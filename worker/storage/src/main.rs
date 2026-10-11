#![cfg_attr(not(all(target_family = "wasm", target_os = "unknown")), no_main)]

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn main() {
    use app::storage::web::worker::StorageWorker;
    use gloo_worker::Registrable;

    StorageWorker::registrar().register();
}
