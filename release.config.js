const CHANNELS = ["alpha", "beta", "stable"];
const channel = process.env.PRERELEASE;

if (!CHANNELS.includes(channel)) {
  throw new Error(
    `PRERELEASE 必须是 ${CHANNELS.join(" / ")} 之一，当前值：${channel ?? "<未设置>"}`,
  );
}

const prerelease = channel === "stable" ? false : channel;

const successComment = [
  "🎉 ${issue.pull_request ? '此 PR 已包含在 ' + nextRelease.version + ' 中' : '此 Issue 已在 ' + nextRelease.version + ' 中解决'} 🎉",
  "项目又向前迈进了一步，感谢每一位贡献者。",
  "发布页：[GitHub Release](${releases.find((r) => r.name === 'GitHub release').url})",
].join("\n\n");

export default {
  branches: [
    "placeholder",
    {
      name: "main",
      prerelease,
    },
  ],
  plugins: [
    ["@semantic-release/commit-analyzer", { preset: "conventionalcommits" }],
    ["@semantic-release/release-notes-generator", { preset: "conventionalcommits" }],
    [
      "@semantic-release/exec",
      {
        verifyReleaseCmd: "echo ${nextRelease.version} > version",
      },
    ],
    [
      "@semantic-release/github",
      {
        assets: ["artifacts/*"],
        successComment,
      },
    ],
  ],
};
