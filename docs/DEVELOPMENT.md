# 开发手册

## 数据库迁移管理

### 生成迁移

```bash
atlas migrate diff [name] --env local
```

- `name` 可选，建议写

### 修改 schema 后

```bash
atlas migrate diff add_xxx --env local
```

### 生成后

更新 `app/src/storage/migration.rs`：

```rust
const MIGRATION_ARRAY: &[rusqlite_migration::M] = &[
    // 按时间从旧到新排列，一行一个迁移版本
    rusqlite_migration::M::up(include_str!("../../../migrations/<timestamp>_initial.sql")),
    rusqlite_migration::M::up(include_str!("../../../migrations/<timestamp>_add_xxx.sql")),
];
```
