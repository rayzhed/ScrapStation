use crate::engine::SourceLoader;
use crate::engine::download_tracker::get_download_folder;
use crate::engine::library_tracker::get_library_folder;
use crate::settings::UserSettings;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex as TokioMutex;

// ===== App storage configuration =====

#[derive(Serialize)]
pub struct AppStorageConfig {
    /// The user-chosen data root (e.g. "D:\MyGames"), or null if using default.
    pub data_root: Option<String>,
    /// Fully resolved download path (absolute) — for UI display.
    pub effective_download_path: String,
    /// Fully resolved library path (absolute) — for UI display.
    pub effective_library_path: String,
    /// Default AppData download path (shown when no custom root is set).
    pub default_download_path: String,
    /// Default AppData library path.
    pub default_library_path: String,
}

/// Return current storage configuration with resolved paths for the UI.
#[tauri::command]
pub async fn get_storage_config(app_handle: AppHandle) -> AppStorageConfig {
    let config = UserSettings::get_app_config();
    let effective_download = get_download_folder(&app_handle).to_string_lossy().to_string();
    let effective_library = get_library_folder(&app_handle).to_string_lossy().to_string();
    let default_base = app_handle.path().app_data_dir().unwrap_or_default();
    let default_download = default_base.join("Downloads").to_string_lossy().to_string();
    let default_library = default_base.join("Library").to_string_lossy().to_string();
    AppStorageConfig {
        data_root: config.data_root,
        effective_download_path: effective_download,
        effective_library_path: effective_library,
        default_download_path: default_download,
        default_library_path: default_library,
    }
}

/// Set a new data root. Pass null to revert to AppData default.
/// App automatically appends \ScrapStation\Downloads and \ScrapStation\Library.
#[tauri::command]
pub async fn set_data_root(root: Option<String>) -> Result<(), String> {
    UserSettings::set_data_root(root)
}

// ===== Known library locations =====

#[derive(Serialize)]
pub struct KnownLibraryLocation {
    /// Absolute path to the Library folder (e.g. D:\Games\ScrapStation\Library)
    pub path: String,
    /// Human-readable label
    pub label: String,
    /// Whether this is the currently active library folder (new installs go here)
    pub is_current: bool,
    /// Whether the user explicitly added this path (can be removed from the list)
    pub removable: bool,
}

/// Return all known library locations: the active one, the AppData default (if different),
/// and any user-added extra paths.
#[tauri::command]
pub async fn get_known_library_locations(app_handle: AppHandle) -> Vec<KnownLibraryLocation> {
    let current_path = get_library_folder(&app_handle);
    let current = current_path.to_string_lossy().to_string();

    let default_base = app_handle.path().app_data_dir().unwrap_or_default();
    let default_lib_path = default_base.join("Library");
    let default_lib = default_lib_path.to_string_lossy().to_string();

    let config = UserSettings::get_app_config();

    // Normalize a path string for comparison (lowercase on Windows, strip trailing sep)
    let normalize = |p: &str| -> String {
        let s = std::path::Path::new(p)
            .to_string_lossy()
            .trim_end_matches(['/', '\\'])
            .to_string();
        #[cfg(windows)]
        { s.to_lowercase() }
        #[cfg(not(windows))]
        { s }
    };

    let current_norm = normalize(&current);

    let mut locations: Vec<KnownLibraryLocation> = Vec::new();

    // Always include the active library folder first
    locations.push(KnownLibraryLocation {
        label: if config.data_root.is_some() { "Current (custom)".to_string() } else { "Default (AppData)".to_string() },
        is_current: true,
        removable: false,
        path: current.clone(),
    });

    // If using a custom root, also expose the default AppData library
    if normalize(&default_lib) != current_norm {
        locations.push(KnownLibraryLocation {
            path: default_lib.clone(),
            label: "Default (AppData)".to_string(),
            is_current: false,
            removable: false,
        });
    }

    // User-added extra library paths
    for extra in &config.extra_library_paths {
        let extra_norm = normalize(extra);
        if extra_norm != current_norm && extra_norm != normalize(&default_lib) {
            locations.push(KnownLibraryLocation {
                path: extra.clone(),
                label: drive_label(extra),
                is_current: false,
                removable: true,
            });
        }
    }

    // Auto-detect ScrapStation\Library on all available drives (Windows)
    #[cfg(windows)]
    for letter in b'A'..=b'Z' {
        let lib = format!("{}:\\ScrapStation\\Library", letter as char);
        let lib_norm = normalize(&lib);
        if lib_norm == current_norm || lib_norm == normalize(&default_lib) {
            continue; // already listed
        }
        // Skip if already covered by an explicit extra path
        if config.extra_library_paths.iter().any(|e| normalize(e) == lib_norm) {
            continue;
        }
        if std::path::Path::new(&lib).exists() {
            locations.push(KnownLibraryLocation {
                path: lib.clone(),
                label: format!("{}:\\ScrapStation", letter as char),
                is_current: false,
                removable: false, // auto-detected from disk, not user-managed
            });
        }
    }

    locations
}

/// Short human-readable label for a library path.
fn drive_label(path: &str) -> String {
    let p = std::path::Path::new(path);
    let parts: Vec<_> = p.components().collect();
    if parts.len() >= 2 {
        format!("{} › {}",
            parts[parts.len()-2].as_os_str().to_string_lossy(),
            parts[parts.len()-1].as_os_str().to_string_lossy())
    } else {
        path.to_string()
    }
}

/// Return all known library folder paths (active + default + extras + auto-detected).
/// Used by repair and normalize operations that need to scan multiple locations.
pub fn all_library_paths(app_handle: &tauri::AppHandle) -> Vec<std::path::PathBuf> {
    use crate::engine::library_tracker::get_library_folder;
    let current = get_library_folder(app_handle);
    let default_base = app_handle.path().app_data_dir().unwrap_or_default();
    let default_lib = default_base.join("Library");
    let config = crate::settings::UserSettings::get_app_config();

    let normalize_pb = |p: &std::path::Path| -> String {
        let s = p.to_string_lossy().trim_end_matches(['/', '\\']).to_string();
        #[cfg(windows)] { s.to_lowercase() }
        #[cfg(not(windows))] { s }
    };

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut paths: Vec<std::path::PathBuf> = Vec::new();

    let mut add = |p: std::path::PathBuf| {
        let key = normalize_pb(&p);
        if seen.insert(key) && p.exists() {
            paths.push(p);
        }
    };

    add(current);
    add(default_lib);
    for extra in &config.extra_library_paths {
        add(std::path::PathBuf::from(extra));
    }

    #[cfg(windows)]
    for letter in b'A'..=b'Z' {
        let lib = std::path::PathBuf::from(format!("{}:\\ScrapStation\\Library", letter as char));
        add(lib);
    }

    paths
}

/// Add a path to the user's extra library locations list.
#[tauri::command]
pub async fn add_library_location(path: String) -> Result<(), String> {
    UserSettings::add_library_path(path)
}

/// Remove a path from the user's extra library locations list.
#[tauri::command]
pub async fn remove_library_location(path: String) -> Result<(), String> {
    UserSettings::remove_library_path(&path)
}

// ===== Per-source settings =====

#[tauri::command]
pub async fn get_source_settings_schema(
    source_id: String,
) -> Result<Vec<crate::config::SettingDefinition>, String> {
    let config = SourceLoader::load_by_id(&source_id)?;
    Ok(config.settings.unwrap_or_default())
}

#[tauri::command]
pub async fn get_source_settings_values(
    source_id: String,
) -> Result<HashMap<String, serde_json::Value>, String> {
    Ok(UserSettings::get_source_settings(&source_id))
}

#[tauri::command]
pub async fn set_source_setting(
    source_id: String,
    setting_id: String,
    value: serde_json::Value,
) -> Result<(), String> {
    UserSettings::set_setting(&source_id, &setting_id, value)
}

#[tauri::command]
pub async fn set_source_settings(
    source_id: String,
    values: HashMap<String, serde_json::Value>,
) -> Result<(), String> {
    UserSettings::set_source_settings(&source_id, values)
}

#[tauri::command]
pub async fn clear_source_settings(source_id: String) -> Result<(), String> {
    UserSettings::clear_source_settings(&source_id)
}

#[tauri::command]
pub async fn get_setting_sections(source_id: String) -> Result<serde_json::Value, String> {
    let config = SourceLoader::load_by_id(&source_id)?;

    if let Some(sections) = config.setting_sections {
        serde_json::to_value(sections).map_err(|e| format!("Failed to serialize sections: {}", e))
    } else {
        Ok(serde_json::json!([]))
    }
}

#[tauri::command]
pub async fn check_storage_condition(
    source_id: String,
    key: String,
) -> Result<serde_json::Value, String> {
    let value = UserSettings::get_setting(&source_id, &key);

    Ok(serde_json::json!({
        "exists": value.is_some(),
        "value": value
    }))
}

// ===== Recovery commands =====

#[derive(Serialize)]
pub struct RecoveryResult {
    pub success: bool,
    pub message: String,
    pub details: Vec<String>,
}

/// Re-run the AppData folder migration: patches `com.scrapstation.app` → `ScrapStation`
/// in all JSON files inside the current AppData directory.
/// Safe to run multiple times — only modifies files that still contain the old string.
#[tauri::command]
pub async fn run_appdata_migration(app_handle: AppHandle) -> RecoveryResult {
    let appdata = match dirs::config_dir() {
        Some(d) => d,
        None => return RecoveryResult {
            success: false,
            message: "Could not locate AppData directory.".into(),
            details: vec![],
        },
    };

    let old_dir = appdata.join("com.scrapstation.app");
    let new_dir = appdata.join("ScrapStation");
    let mut details: Vec<String> = vec![];

    if !old_dir.exists() {
        return RecoveryResult {
            success: true,
            message: "No old data folder found — nothing to migrate.".into(),
            details,
        };
    }

    // Merge-copy: copy files from old → new only when the destination file is
    // absent or empty. This handles the common case where Tauri auto-created
    // the new folder on first launch (making !new_dir.exists() false) but the
    // actual data files (library.json, downloads.json, …) were never copied.
    let moved = move_missing_files(&old_dir, &new_dir);
    match moved {
        Ok(n) if n > 0 => details.push(format!("Moved {} file(s) from old folder.", n)),
        Ok(_) => details.push("All files already present in new folder.".into()),
        Err(e) => return RecoveryResult {
            success: false,
            message: format!("Failed to copy files: {}", e),
            details,
        },
    }

    // Patch identifier string in all JSON files under the new AppData folder
    let target = app_handle.path().app_data_dir().unwrap_or(new_dir);
    let patched = patch_and_count(&target, "com.scrapstation.app", "ScrapStation");
    if patched > 0 {
        details.push(format!("Patched {} JSON file(s) with corrected paths.", patched));
    } else {
        details.push("No outdated path references found in JSON files.".into());
    }

    RecoveryResult {
        success: true,
        message: "Migration complete.".into(),
        details,
    }
}

/// Normalize all in-memory download and library paths from absolute to relative,
/// and fix any references to the old AppData folder name.
/// Saves both databases after fixing.
#[tauri::command]
pub async fn fix_broken_paths(
    download_tracker: tauri::State<'_, Arc<TokioMutex<crate::engine::DownloadTracker>>>,
    library_tracker: tauri::State<'_, Arc<TokioMutex<crate::engine::LibraryTracker>>>,
) -> Result<RecoveryResult, String> {
    let mut details: Vec<String> = vec![];

    let dl_fixed = {
        let tracker = download_tracker.lock().await;
        tracker.normalize_paths().await
    };
    if dl_fixed > 0 {
        details.push(format!("Fixed {} download path(s).", dl_fixed));
    } else {
        details.push("Download paths are already correct.".into());
    }

    let lib_fixed = {
        let tracker = library_tracker.lock().await;
        tracker.normalize_paths().await
    };
    if lib_fixed > 0 {
        details.push(format!("Fixed {} library path(s).", lib_fixed));
    } else {
        details.push("Library paths are already correct.".into());
    }

    Ok(RecoveryResult {
        success: true,
        message: if dl_fixed + lib_fixed > 0 {
            format!("Fixed {} total path reference(s).", dl_fixed + lib_fixed)
        } else {
            "All paths are already correct — nothing to fix.".into()
        },
        details,
    })
}

/// Move files from `src` into `dst`, skipping files already present and non-empty
/// in `dst`. Returns the number of files moved. Uses rename (instant) with
/// copy+delete fallback for cross-drive moves.
fn move_missing_files(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<usize> {
    std::fs::create_dir_all(dst)?;
    let mut count = 0;
    for entry in std::fs::read_dir(src)?.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            count += move_missing_files(&src_path, &dst_path)?;
            let _ = std::fs::remove_dir(&src_path);
        } else {
            let dst_empty = dst_path.metadata().map(|m| m.len() == 0).unwrap_or(true);
            if !dst_path.exists() || dst_empty {
                if std::fs::rename(&src_path, &dst_path).is_err() {
                    std::fs::copy(&src_path, &dst_path)?;
                    std::fs::remove_file(&src_path)?;
                }
                count += 1;
            }
        }
    }
    Ok(count)
}

fn patch_and_count(dir: &std::path::Path, old: &str, new: &str) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else { return 0 };
    let mut count = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            count += patch_and_count(&path, old, new);
        } else if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if content.contains(old) {
                    let _ = std::fs::write(&path, content.replace(old, new));
                    count += 1;
                }
            }
        }
    }
    count
}
