env "local" {
  dev = "sqlite://file?mode=memory"
  src = "file://schema.lt.hcl"
  migration {
    dir = "file://crates/storage-migrations/sql"
  }
}
