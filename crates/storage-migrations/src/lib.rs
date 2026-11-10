pub const MIGRATIONS: rusqlite_migration::Migrations =
    rusqlite_migration::Migrations::from_slice(MIGRATION_ARRAY);

const MIGRATION_ARRAY: &[rusqlite_migration::M] = &[rusqlite_migration::M::up(include_str!(
    "../sql/20261026015207_initial.sql"
))];
