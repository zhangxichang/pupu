#[cfg(not(target_family = "wasm"))]
fn main() {}

#[cfg(all(target_os = "unknown", target_arch = "wasm32"))]
fn main() {}
