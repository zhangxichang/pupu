const MIGRATION_ARRAY: &[rusqlite_migration::M] = &[rusqlite_migration::M::up(include_str!(
    "../../../migrations/20260730044855.sql"
))];
pub const MIGRATIONS: rusqlite_migration::Migrations =
    rusqlite_migration::Migrations::from_slice(MIGRATION_ARRAY);
