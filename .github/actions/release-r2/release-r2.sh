#!/usr/bin/env bash
set -euo pipefail

ACCOUNT_ID="${account_id:-${CLOUDFLARE_ACCOUNT_ID:-}}"
API_TOKEN="${api_token:-${CLOUDFLARE_API_TOKEN:-}}"
FILES_RAW="${r2_files:-${R2_FILES:-}}"
BUCKET="${R2_BUCKET:-pupu-apac}"

: "${ACCOUNT_ID:?account_id 或 CLOUDFLARE_ACCOUNT_ID 不能为空}"
: "${API_TOKEN:?api_token 或 CLOUDFLARE_API_TOKEN 不能为空}"
: "${FILES_RAW:?r2_files 不能为空}"

export CLOUDFLARE_ACCOUNT_ID="$ACCOUNT_ID"
export CLOUDFLARE_API_TOKEN="$API_TOKEN"

BUILD_ID="${GITHUB_SHA::12}-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}"
echo "BUILD_ID=$BUILD_ID"

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    echo "build_id=$BUILD_ID" >> "$GITHUB_OUTPUT"
fi

# r2_files 是一行一个文件路径，支持 glob，取第一个匹配
mapfile -t FILE_PATTERNS <<< "$FILES_RAW"

MANIFEST=$(mktemp)
KEEP=$(mktemp)
OBJECTS=$(mktemp)
LIVE_MANIFEST=$(mktemp)
trap 'rm -f "$MANIFEST" "$KEEP" "$OBJECTS" "$LIVE_MANIFEST"' EXIT

# R2 文件不进入 Worker 静态资源；模式来自 r2_files 的路径
: > dist/.assetsignore

keys=()
for path_pattern in "${FILE_PATTERNS[@]}"; do
    [[ -n "$path_pattern" ]] || continue

    ignore_pattern="${path_pattern#./}"
    ignore_pattern="${ignore_pattern#dist/}"
    printf '%s\n' "$ignore_pattern" >> dist/.assetsignore

    matches=()
    while IFS= read -r match; do
        [[ -n "$match" ]] || continue
        [[ -f "$match" ]] || continue
        matches+=("$match")
    done < <(compgen -G "$path_pattern" || true)

    if [[ "${#matches[@]}" -eq 0 ]]; then
        echo "没有匹配的文件: $path_pattern" >&2
        exit 1
    fi

    # 取第一个匹配
    path="${matches[0]}"
    key="$(basename "$path")"

    keys+=("$key")
    echo "上传: $key"
    npx --yes wrangler@4.35.0 r2 object put "$BUCKET/$key" \
        --file "$path" \
        --remote
done

if [[ "${#keys[@]}" -eq 0 ]]; then
    echo "没有可上传的文件" >&2
    exit 1
fi

jq -n \
    --arg build_id "$BUILD_ID" \
    --arg created_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --arg commit "$GITHUB_SHA" \
    --arg run_id "$GITHUB_RUN_ID" \
    --args \
    '{
        build_id: $build_id,
        created_at: $created_at,
        commit: $commit,
        run_id: $run_id,
        files: $ARGS.positional
    }' \
    "${keys[@]}" > "$MANIFEST"

echo "写 manifest: _releases/$BUILD_ID.json"
npx --yes wrangler r2 object put "$BUCKET/_releases/$BUILD_ID.json" \
    --file "$MANIFEST" \
    --remote

R2_API="https://api.cloudflare.com/client/v4/accounts/$CLOUDFLARE_ACCOUNT_ID/r2/buckets/$BUCKET/objects"

# 当前候选 release 的所有文件
jq -r '.files[]' "$MANIFEST" >> "$KEEP"

# 读取线上 deployment 的 BUILD_ID tag
LIVE_BUILD_ID=""
if STATUS_OUTPUT=$(NO_COLOR=1 npx --yes wrangler deployments status 2>&1); then
    LIVE_BUILD_ID=$(printf '%s\n' "$STATUS_OUTPUT" \
        | sed -n 's/.*Tag:[[:space:]]*//p' \
        | head -n1 \
        | tr -d '[:space:]')
fi

if [[ -z "$LIVE_BUILD_ID" ]]; then
    echo "线上没有 BUILD_ID tag，跳过清理" >&2
    exit 0
fi

# 线上 release 的 manifest
if ! curl -fsS --retry 3 -o "$LIVE_MANIFEST" \
    -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
    "$R2_API/_releases/$LIVE_BUILD_ID.json"; then
    echo "线上 manifest 缺失，跳过清理" >&2
    exit 0
fi

if ! jq -e '(.files | type == "array") and (.files | length > 0)' \
    "$LIVE_MANIFEST" >/dev/null; then
    echo "线上 manifest 无效，跳过清理" >&2
    exit 0
fi

jq -r '.files[]' "$LIVE_MANIFEST" >> "$KEEP"

# 只保留当前和线上的 manifest 文件本身
{
    printf '%s\n' "_releases/$BUILD_ID.json"
    printf '%s\n' "_releases/$LIVE_BUILD_ID.json"
} >> "$KEEP"

sort -u "$KEEP" -o "$KEEP"

# 此 action 假设 BUCKET 专用于发布文件。
# 只保留当前/线上 manifest 及其文件；其他对象（含旧 manifest）都删除。
# 列出 R2 所有对象
cursor=""
while :; do
    args=(--get "$R2_API" --data-urlencode "per_page=1000")
    [[ -z "$cursor" ]] || args+=(--data-urlencode "cursor=$cursor")

    if ! response=$(curl -fsS --retry 3 "${args[@]}" \
        -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN"); then
        echo "列出 R2 对象失败，跳过清理" >&2
        exit 0
    fi

    if ! jq -e '.success' <<<"$response" >/dev/null; then
        echo "R2 返回失败，跳过清理" >&2
        exit 0
    fi

    if ! jq -r '.result[] | .key' <<<"$response" >> "$OBJECTS"; then
        echo "解析 R2 列表失败，跳过清理" >&2
        exit 0
    fi

    cursor=$(jq -r '.result_info.cursor // empty' <<<"$response") || break
    truncated=$(jq -r '.result_info.is_truncated // false' <<<"$response") || break
    [[ "$truncated" == true && -n "$cursor" ]] || break
done

# 删除不在保留集合里的文件
while IFS= read -r key; do
    [[ -n "$key" ]] || continue

    if grep -Fqx -- "$key" "$KEEP"; then
        echo "保留: $key"
        continue
    fi

    echo "删除: $key"
    if ! curl -fsS --retry 3 -X DELETE "$R2_API/$key" \
        -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" |
        jq -e '.success' >/dev/null; then
        echo "删除失败: $key" >&2
    fi
done < "$OBJECTS"

echo "清理完成"
