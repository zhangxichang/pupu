# 数据库迁移

迁移 SQL 放在 `sql/`，由 Atlas 根据仓库根目录的 `schema.lt.hcl` 生成；运行时通过 `rusqlite_migration` 执行。

## 生成迁移

修改 `schema.lt.hcl` 后，在仓库根目录执行：

```bash
atlas migrate diff add_xxx --env local
```

`add_xxx` 是迁移名，建议写上。生成的 SQL 会放到 `crates/storage-migrations/sql/`。

## 加入运行时迁移列表

生成后，更新 `crates/storage-migrations/src/lib.rs` 的 `MIGRATION_ARRAY`，按时间从旧到新追加：

```rust
const MIGRATION_ARRAY: &[rusqlite_migration::M] = &[
    rusqlite_migration::M::up(include_str!("../sql/<timestamp>_initial.sql")),
    rusqlite_migration::M::up(include_str!("../sql/<timestamp>_add_xxx.sql")),
];
```

原生端 `crates/storage/src/native.rs` 和 Web 端 `web/wasm/storage-worker` 在打开数据库时都会调用 `MIGRATIONS.to_latest`。

## 校验

```bash
atlas migrate validate --env local
```

## 注意

- 不要修改已经执行过的迁移文件，也不要手动改 `atlas.sum`。
- 新迁移只能追加到 `MIGRATION_ARRAY` 末尾，顺序必须与 `sql/` 中的时间戳一致。
