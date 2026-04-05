mod config;
mod constants;
mod engine;
mod utils;
mod api;
mod scraper;
mod types;
pub mod settings;
pub mod auth;

use std::sync::Arc;
use tauri::{Builder, Emitter, Manager};
use tokio::sync::Mutex as TokioMutex;
use engine::{DownloadTracker, LibraryTracker, SourceLoader};

/// One-time migration: if the old `com.scrapstation.app` AppData folder exists
/// but the new `ScrapStation` folder does not, copy everything over and patch
/// the old identifier string inside the JSON database files so stored paths
/// resolve correctly under the new folder name.
fn run_startup_migration() {
    let Some(appdata) = dirs::config_dir() else { return };

    let old_dir = appdata.join("com.scrapstation.app");
    let new_dir = appdata.join("ScrapStation");

    if !old_dir.exists() {
        return; // Nothing to do
    }

    log::info!("[Migration] Migrating data from {:?} to {:?}", old_dir, new_dir);

    // Merge-copy: only copies files that are absent or empty in the destination.
    // Handles the case where Tauri auto-created the new folder on first launch.
    if let Err(e) = move_missing_files(&old_dir, &new_dir) {
        log::error!("[Migration] File move failed: {}", e);
        return;
    }

    // Patch old identifier in JSON DB files so stored absolute paths still resolve
    patch_identifier_in_json(&new_dir, "com.scrapstation.app", "ScrapStation");

    // Leave a breadcrumb so users know what happened (old folder kept as backup)
    let _ = std::fs::write(
        old_dir.join("MIGRATED_TO_ScrapStation.txt"),
        "Your ScrapStation data has been moved to %APPDATA%\\ScrapStation\n\
         This folder is no longer used and can be safely deleted.",
    );

    log::info!("[Migration] Migration complete — old folder kept as backup at {:?}", old_dir);
}

/// Move files from `src` into `dst`, skipping files already present and non-empty
/// in `dst`. Uses rename (instant) and falls back to copy+delete across drives.
fn move_missing_files(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            move_missing_files(&src_path, &dst_path)?;
            // Remove directory if now empty
            let _ = std::fs::remove_dir(&src_path);
        } else {
            let dst_empty = dst_path.metadata().map(|m| m.len() == 0).unwrap_or(true);
            if !dst_path.exists() || dst_empty {
                if std::fs::rename(&src_path, &dst_path).is_err() {
                    // Cross-drive fallback
                    std::fs::copy(&src_path, &dst_path)?;
                    std::fs::remove_file(&src_path)?;
                }
            }
        }
    }
    Ok(())
}

pub fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// Walk `dir` looking for `.json` files and replace occurrences of `old_id`
/// with `new_id` in their contents (handles stored absolute paths).
fn patch_identifier_in_json(dir: &std::path::Path, old_id: &str, new_id: &str) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            patch_identifier_in_json(&path, old_id, new_id);
        } else if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if content.contains(old_id) {
                    let patched = content.replace(old_id, new_id);
                    let _ = std::fs::write(&path, patched);
                    log::info!("[Migration] Patched identifiers in {:?}", path);
                }
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Migrate data from old AppData folder name before anything else touches the filesystem
    run_startup_migration();

    // Initialize logging:
    // - Our crate: debug level
    // - Everything else: warn (silences selectors::matching, html5ever, hyper, etc.)
    // Override with RUST_LOG env var if set.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(
        "warn,scrapstation_lib=debug",
    ))
    .format_timestamp_millis()
    .format(|buf, record| {
        use std::io::Write;
        // Compact format: timestamp LEVEL [target] message
        let level_style = buf.default_level_style(record.level());
        writeln!(
            buf,
            "{} {level_style}{:5}{level_style:#} [{}] {}",
            buf.timestamp_millis(),
            record.level(),
            record.target(),
            record.args()
        )
    })
    .init();

    log::info!("{} v{} starting...", crate::constants::APP_NAME, crate::constants::APP_VERSION);

    Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Initialize download tracker
            let download_tracker = DownloadTracker::new(app.handle().clone());
            let download_tracker = Arc::new(TokioMutex::new(download_tracker));

            // Load persisted downloads
            let tracker_clone = download_tracker.clone();
            tauri::async_runtime::spawn(async move {
                let tracker = tracker_clone.lock().await;
                if let Err(e) = tracker.load().await {
                    log::error!("[Setup] Failed to load downloads: {}", e);
                }
            });

            // Store download tracker in app state
            app.manage(download_tracker);
            log::info!("[Setup] Download tracker initialized");

            // Initialize library tracker
            let library_tracker = LibraryTracker::new(app.handle().clone());
            let library_tracker = Arc::new(TokioMutex::new(library_tracker));

            // Load persisted library
            let library_clone = library_tracker.clone();
            tauri::async_runtime::spawn(async move {
                let tracker = library_clone.lock().await;
                if let Err(e) = tracker.load().await {
                    log::error!("[Setup] Failed to load library: {}", e);
                }
            });

            // Store library tracker in app state
            app.manage(library_tracker);
            log::info!("[Setup] Library tracker initialized");

            // Watch sources directory for YAML changes → emit sources-changed
            let watcher_handle = app.handle().clone();
            let sources_dir = SourceLoader::sources_dir();
            if !sources_dir.exists() {
                let _ = std::fs::create_dir_all(&sources_dir);
            }
            std::thread::spawn(move || {
                use notify::{Watcher, RecommendedWatcher, RecursiveMode, Config};
                use std::sync::mpsc::channel;
                use std::time::{Duration, Instant};

                let (tx, rx) = channel();
                let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
                    Ok(w) => w,
                    Err(e) => { log::error!("[SourceWatcher] Failed to create watcher: {}", e); return; }
                };
                if let Err(e) = watcher.watch(&sources_dir, RecursiveMode::NonRecursive) {
                    log::error!("[SourceWatcher] Failed to watch sources dir: {}", e);
                    return;
                }
                log::info!("[SourceWatcher] Watching {:?}", sources_dir);

                let mut last_emit = Instant::now() - Duration::from_secs(10);
                for res in rx {
                    match res {
                        Ok(event) => {
                            let is_yaml = event.paths.iter().any(|p| {
                                p.extension().map(|e| e == "yaml" || e == "yml").unwrap_or(false)
                            });
                            if is_yaml && last_emit.elapsed() > Duration::from_millis(400) {
                                last_emit = Instant::now();
                                let _ = watcher_handle.emit("sources-changed", ());
                                log::info!("[SourceWatcher] Emitted sources-changed");
                            }
                        }
                        Err(e) => log::warn!("[SourceWatcher] watch error: {}", e),
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            api::install_source_config,
            api::install_source_from_path,
            api::list_sources,
            api::load_dynamic_source,
            api::search_dynamic_source,
            api::get_source_metadata,
            api::get_sources_folder_path,
            api::open_sources_folder,
            api::get_game_detail_sections,
            api::open_url_in_browser,
            api::resolve_download_link,
            api::download_file,
            api::detect_host,
            api::smart_download,
            api::navigate_link,
            api::resolve_button_link,
            api::get_source_settings_schema,
            api::get_source_settings_values,
            api::set_source_setting,
            api::set_source_settings,
            api::clear_source_settings,
            api::get_setting_sections,
            api::execute_action,
            api::check_storage_condition,
            auth::webview_auth::open_auth_window,
            auth::webview_auth::login_with_credentials,
            auth::webview_auth::logout,
            auth::webview_auth::get_auth_status,
            auth::webview_auth::set_manual_cookies,
            auth::webview_auth::fetch_authenticated,
            auth::webview_auth::extract_webview_cookies,
            api::source_login,
            api::source_register,
            api::source_logout,
            api::check_auth_status,
            api::start_sso_login,
            api::set_source_cookies,
            api::get_source_cookies,
            api::clear_source_cookies,
            api::fetch_image,
            api::read_local_image,
            api::estimate_total_pages,
            // Download tracker commands
            api::register_download,
            api::update_download_status,
            api::get_downloads,
            api::pause_download,
            api::resume_download,
            api::cancel_download,
            api::remove_download,
            api::clear_finished_downloads,
            api::get_download_folder_path,
            api::open_download_folder,
            api::open_file_location,
            // Library commands
            api::get_library_games,
            api::get_library_game,
            api::add_game_to_library,
            api::remove_from_library,
            api::link_download_to_library,
            api::extract_to_library,
            api::launch_game,
            api::set_game_executable,
            api::open_game_folder,
            api::get_library_folder_path,
            api::open_library_folder,
            api::move_game,
            api::rescan_game_executables,
            api::is_game_in_library,
            api::get_library_game_id,
            api::update_archive_password,
            api::cache_game_cover,
            api::cache_missing_covers,
            api::update_game_cover_url,
            api::get_install_preflight,
            api::probe_download_size,
            api::download_and_run_installer,
            api::repair_library,
            // Storage configuration
            api::get_storage_config,
            api::set_data_root,
            api::get_known_library_locations,
            api::add_library_location,
            api::remove_library_location,
            // Recovery commands
            api::run_appdata_migration,
            api::fix_broken_paths,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}