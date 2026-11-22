use std::{fs, path::Path};

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if matches!(target_os.as_str(), "windows" | "linux" | "android") {
        let version_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../version");
        println!("cargo:rerun-if-changed={}", version_path.display());
        let version = fs::read_to_string(&version_path)
            .ok()
            .map(|version| version.trim().to_string())
            .filter(|version| !version.is_empty())
            .unwrap_or_else(|| "dev-version".to_string());
        println!("cargo:rustc-env=APP_VERSION={version}");
    }
    let lucide = lucide_slint::lib();
    let target_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/lib");
    fs::create_dir_all(&target_dir).expect("create ui/lib directory");
    fs::copy(&lucide, target_dir.join("lucide.slint")).expect("copy lucide.slint");
    println!("cargo:rerun-if-changed={lucide}");
    slint_build::compile("ui/viewport.slint").unwrap();
}
