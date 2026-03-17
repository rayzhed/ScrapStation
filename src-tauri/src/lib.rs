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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            api::rescan_game_executables,
            api::is_game_in_library,
            api::get_library_game_id,
            api::update_archive_password,
            api::cache_game_cover,
            api::cache_missing_covers,
            api::update_game_cover_url,
            api::get_install_preflight,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}