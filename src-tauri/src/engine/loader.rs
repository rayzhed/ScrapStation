use crate::config::SourceConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::env;

pub struct SourceLoader;

impl SourceLoader {
    /// Returns the sources directory for public use (e.g. from commands).
    pub fn sources_dir() -> PathBuf {
        Self::get_sources_dir()
    }

    // Get the path to sources directory
    fn get_sources_dir() -> PathBuf {
        // In development (cargo run): use src-tauri/sources/ directly
        if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
            return PathBuf::from(manifest_dir).join("sources");
        }

        // In production: store sources in the user's config directory so they
        // remain accessible after packaging (src-tauri is not available in a
        // compiled .exe).
        let sources_dir = if let Some(config_dir) = dirs::config_dir() {
            config_dir.join(crate::constants::APP_NAME).join("sources")
        } else {
            // Last-resort fallback
            env::current_dir().unwrap_or_default().join("sources")
        };

        if !sources_dir.exists() {
            let _ = fs::create_dir_all(&sources_dir);
        }

        sources_dir
    }

    pub fn load_all() -> Result<Vec<SourceConfig>, String> {
        let sources_dir = Self::get_sources_dir();

        log::info!("Looking for sources in: {}", sources_dir.display());

        if !sources_dir.exists() {
            log::info!("Sources directory does not exist yet: {}", sources_dir.display());
            return Ok(Vec::new());
        }

        let mut configs = Vec::new();

        for entry in fs::read_dir(&sources_dir)
            .map_err(|e| format!("Failed to read sources directory: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            // Load ALL .yaml files except schema.yaml
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

                // Skip schema, example reference files, and files starting with _
                if filename != "schema"
                    && !filename.starts_with("example")
                    && !filename.starts_with('_')
                {
                    match Self::load_from_file(&path) {
                        Ok(config) => {
                            log::info!("Loaded source: {} (id: {})", config.name, config.id);
                            configs.push(config);
                        }
                        Err(e) => {
                            log::warn!("Failed to load {}: {}", path.display(), e);
                            // Continue instead of failing to load other sources
                        }
                    }
                }
            }
        }

        log::info!("Total sources loaded: {}", configs.len());
        Ok(configs)
    }

    pub fn load_by_id(id: &str) -> Result<SourceConfig, String> {
        let sources_dir = Self::get_sources_dir();
        let path = sources_dir.join(format!("{}.yaml", id));

        log::info!("Loading source by ID '{}' from: {}", id, path.display());

        Self::load_from_file(&path)
    }

    fn load_from_file(path: &Path) -> Result<SourceConfig, String> {
        let contents = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file {:?}: {}", path, e))?;

        let config: SourceConfig = serde_yaml::from_str(&contents)
            .map_err(|e| format!("Failed to parse YAML from {:?}: {}", path, e))?;

        // Verify that filename matches the ID in YAML
        if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
            if filename != config.id {
                log::warn!("Filename '{}' doesn't match config id '{}'", filename, config.id);
            }
        }

        Ok(config)
    }
}