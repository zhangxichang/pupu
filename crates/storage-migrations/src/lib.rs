pub const MIGRATIONS: rusqlite_migration::Migrations =
    rusqlite_migration::Migrations::from_slice(MIGRATION_ARRAY);

const MIGRATION_ARRAY: &[rusqlite_migration::M] = &[
    rusqlite_migration::M::up(include_str!("../sql/20261116194729.sql")),
    rusqlite_migration::M::up(include_str!("../sql/20261119212028_iroh_network.sql"))
        .foreign_key_check(),
    rusqlite_migration::M::up(include_str!("../sql/20261119225138_iroh_multi_account.sql"))
        .foreign_key_check(),
];
