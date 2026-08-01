fn main() {
    // Create empty stub files for any bundled sources that don't exist locally.
    // This lets `include_str!` compile in CI without the real (gitignored) configs.
    // The stubs are empty strings, so seed_default_sources will skip them at runtime.
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let sources_dir = std::path::Path::new(&manifest).join("sources");
    for name in &["steamrip.yaml", "onlinefix.yaml"] {
        let path = sources_dir.join(name);
        if !path.exists() {
            let _ = std::fs::create_dir_all(&sources_dir);
            let _ = std::fs::write(&path, "");
        }
    }

    tauri_build::build()
}
