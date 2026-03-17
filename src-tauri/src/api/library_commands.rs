use crate::engine::library_tracker::{
    LibraryTracker, LibraryGame, LibraryGameStatus, GameExecutable,
    get_library_folder, get_game_folder, generate_library_game_id, generate_game_slug, current_timestamp
};
use crate::engine::archive_extractor::{ArchiveExtractor, delete_archives};
use crate::engine::executable_detector::ExecutableDetector;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State, Emitter};
use tokio::sync::Mutex as TokioMutex;

// ========== LIBRARY CRUD ==========

/// Get all games in the library
#[tauri::command]
pub async fn get_library_games(
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
) -> Result<Vec<LibraryGame>, String> {
    let tracker = tracker.lock().await;
    Ok(tracker.get_all_games().await)
}

/// Get a specific game from the library
#[tauri::command]
pub async fn get_library_game(
    id: String,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
) -> Result<Option<LibraryGame>, String> {
    let tracker = tracker.lock().await;
    Ok(tracker.get_game(&id).await)
}

/// Add a game to the library (called when download starts)
#[tauri::command]
pub async fn add_game_to_library(
    source_slug: String,
    source_game_id: String,
    title: String,
    cover_url: Option<String>,
    archive_password: Option<String>,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
    app_handle: AppHandle,
) -> Result<String, String> {
    log::info!("[Library] add_game_to_library called:");
    log::info!("[Library]   source_slug: {}", source_slug);
    log::info!("[Library]   source_game_id: {}", source_game_id);
    log::info!("[Library]   title: {}", title);
    log::info!("[Library]   archive_password: {:?}", archive_password);

    let id = generate_library_game_id(&source_slug, &source_game_id);
    let game_slug = generate_game_slug(&title);
    let install_path = get_game_folder(&app_handle, &game_slug)
        .join("game")
        .to_string_lossy()
        .to_string();

    log::info!("[Library]   Generated ID: {}", id);
    log::info!("[Library]   Generated slug: {}", game_slug);
    log::info!("[Library]   Install path: {}", install_path);

    let game = LibraryGame {
        id: id.clone(),
        source_slug,
        source_game_id,
        title,
        cover_url: cover_url.clone(),
        cover_path: None,
        install_path,
        install_size: 0,
        status: LibraryGameStatus::Downloading,
        installed_at: current_timestamp(),
        last_played: None,
        total_playtime: 0,
        executables: vec![],
        primary_exe: None,
        archive_password,
        download_ids: vec![],
    };

    let tracker_arc = tracker.inner().clone();
    {
        let t = tracker_arc.lock().await;
        t.add_game(game).await?;
    }

    // Spawn cover caching in background — doesn't block the command response
    if let Some(url) = cover_url {
        let app_clone = app_handle.clone();
        let id_clone = id.clone();
        let tracker_clone = tracker_arc.clone();
        tokio::spawn(async move {
            if let Err(e) = cache_cover_impl(&app_clone, &id_clone, &url, tracker_clone).await {
                log::warn!("[Library] Cover caching failed for {}: {}", id_clone, e);
            }
        });
    }

    log::info!("[Library] Added game to library: {}", id);
    Ok(id)
}

/// Remove a game from the library
#[tauri::command]
pub async fn remove_from_library(
    id: String,
    delete_files: bool,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
) -> Result<(), String> {
    let tracker = tracker.lock().await;

    // Get game info before removing
    let game = tracker.get_game(&id).await;

    // Remove from database
    tracker.remove_game(&id).await?;

    // Delete files if requested
    if delete_files {
        if let Some(g) = game {
            let install_path = PathBuf::from(&g.install_path);
            // Delete the game folder (parent of install_path which is /game)
            if let Some(game_folder) = install_path.parent() {
                if game_folder.exists() {
                    tokio::fs::remove_dir_all(game_folder)
                        .await
                        .map_err(|e| format!("Failed to delete game folder: {}", e))?;
                    log::info!("[Library] Deleted game folder: {:?}", game_folder);
                }
            }
        }
    }

    log::info!("[Library] Removed game from library: {}", id);
    Ok(())
}

/// Link a download ID to a library game
#[tauri::command]
pub async fn link_download_to_library(
    game_id: String,
    download_id: String,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
) -> Result<(), String> {
    let tracker = tracker.lock().await;
    tracker.add_download_id(&game_id, &download_id).await
}

// ========== EXTRACTION ==========

/// Extract archives to library and set up the game
#[tauri::command]
pub async fn extract_to_library(
    app_handle: AppHandle,
    game_id: String,
    archive_paths: Vec<String>,
    password: Option<String>,
    library_tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
) -> Result<(), String> {
    log::info!("[Library] *** EXTRACT_TO_LIBRARY CALLED ***");
    log::info!("[Library] Game ID: {}", game_id);
    log::info!("[Library] Archive paths: {:?}", archive_paths);
    log::info!("[Library] Password from param: {:?}", password);

    // Get the game from library
    let game = {
        let tracker = library_tracker.lock().await;
        tracker.get_game(&game_id).await
            .ok_or_else(|| format!("Game not found in library: {}", game_id))?
    };

    log::info!("[Library] Found game in library:");
    log::info!("[Library]   Title: {}", game.title);
    log::info!("[Library]   Install path: {}", game.install_path);
    log::info!("[Library]   Password stored in game: {:?}", game.archive_password);

    // Update status to extracting
    {
        let tracker = library_tracker.lock().await;
        tracker.update_status(&game_id, LibraryGameStatus::Extracting).await?;
    }

    // Determine password (from param, game metadata, or none)
    let extraction_password = password.or(game.archive_password.clone());
    log::info!("[Library] Final extraction password: {}", if extraction_password.is_some() { "[SET]" } else { "[NONE]" });

    // Convert paths
    let paths: Vec<PathBuf> = archive_paths.iter().map(PathBuf::from).collect();

    // Determine destination
    let destination = PathBuf::from(&game.install_path);

    // Create extractor and run extraction
    let extractor = ArchiveExtractor::new(app_handle.clone());

    let result = extractor.extract(
        paths.clone(),
        destination.clone(),
        extraction_password,
        game_id.clone(),
    ).await;

    match result {
        Ok(extraction_result) => {
            log::info!("[Library] Extraction completed: {} files, {} bytes",
                extraction_result.files_extracted, extraction_result.total_size);

            // Detect executables
            let executables = ExecutableDetector::detect_executables(&destination, &game.title);
            log::info!("[Library] Detected {} executables", executables.len());

            // Calculate install size
            let install_size = ExecutableDetector::calculate_directory_size(&destination);

            // Update library game
            {
                let tracker = library_tracker.lock().await;
                tracker.set_executables(&game_id, executables).await?;
                tracker.set_install_size(&game_id, install_size).await?;
                tracker.update_status(&game_id, LibraryGameStatus::Ready).await?;
            }

            // Delete archive files
            if let Err(e) = delete_archives(&paths).await {
                log::warn!("[Library] Failed to delete archives: {}", e);
            }

            // Emit success event
            let _ = app_handle.emit("extraction-complete", serde_json::json!({
                "gameId": game_id,
                "success": true
            }));

            Ok(())
        }
        Err(e) => {
            log::error!("[Library] Extraction failed: {}", e);

            // Update status to corrupted
            {
                let tracker = library_tracker.lock().await;
                tracker.update_status(&game_id, LibraryGameStatus::Corrupted).await?;
            }

            // Emit error event
            let _ = app_handle.emit("extraction-error", serde_json::json!({
                "gameId": game_id,
                "error": e.to_string()
            }));

            Err(e.to_string())
        }
    }
}

// ========== GAME LAUNCH ==========

/// Launch a game
#[tauri::command]
pub async fn launch_game(
    id: String,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
) -> Result<(), String> {
    let game = {
        let tracker = tracker.lock().await;
        tracker.get_game(&id).await
            .ok_or_else(|| format!("Game not found: {}", id))?
    };

    if game.status != LibraryGameStatus::Ready {
        return Err("Game is not ready to launch".to_string());
    }

    let exe_path = game.primary_exe
        .ok_or_else(|| "No executable selected".to_string())?;

    let full_exe_path = PathBuf::from(&game.install_path).join(&exe_path);

    if !full_exe_path.exists() {
        return Err(format!("Executable not found: {:?}", full_exe_path));
    }

    log::info!("[Library] Launching game: {:?}", full_exe_path);

    // Get working directory (same as exe directory)
    let working_dir = full_exe_path.parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    // Launch the game
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new(&full_exe_path)
            .current_dir(working_dir)
            .spawn()
            .map_err(|e| format!("Failed to launch game: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new(&full_exe_path)
            .current_dir(working_dir)
            .spawn()
            .map_err(|e| format!("Failed to launch game: {}", e))?;
    }

    // Update last played
    {
        let tracker = tracker.lock().await;
        tracker.update_playtime(&id, 0).await?; // Just updates last_played
    }

    Ok(())
}

/// Set the primary executable for a game
#[tauri::command]
pub async fn set_game_executable(
    id: String,
    executable_path: String,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
) -> Result<(), String> {
    let tracker = tracker.lock().await;
    tracker.set_primary_exe(&id, &executable_path).await
}

// ========== UTILITY ==========

/// Open game folder in file explorer
#[tauri::command]
pub async fn open_game_folder(
    id: String,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
) -> Result<(), String> {
    let game = {
        let tracker = tracker.lock().await;
        tracker.get_game(&id).await
            .ok_or_else(|| format!("Game not found: {}", id))?
    };

    let folder = PathBuf::from(&game.install_path);

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&folder)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&folder)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&folder)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    Ok(())
}

/// Get library folder path
#[tauri::command]
pub async fn get_library_folder_path(
    app_handle: AppHandle,
) -> Result<String, String> {
    Ok(get_library_folder(&app_handle).to_string_lossy().to_string())
}

/// Rescan executables for a game
#[tauri::command]
pub async fn rescan_game_executables(
    id: String,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
) -> Result<Vec<GameExecutable>, String> {
    let game = {
        let tracker = tracker.lock().await;
        tracker.get_game(&id).await
            .ok_or_else(|| format!("Game not found: {}", id))?
    };

    let destination = PathBuf::from(&game.install_path);
    let executables = ExecutableDetector::detect_executables(&destination, &game.title);

    {
        let tracker = tracker.lock().await;
        tracker.set_executables(&id, executables.clone()).await?;
    }

    Ok(executables)
}

/// Check if a game is in the library by source info
#[tauri::command]
pub async fn is_game_in_library(
    source_slug: String,
    source_game_id: String,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
) -> Result<Option<LibraryGame>, String> {
    let tracker = tracker.lock().await;
    Ok(tracker.find_by_source(&source_slug, &source_game_id).await)
}

/// Update archive password for a library game
#[tauri::command]
pub async fn update_archive_password(
    game_id: String,
    password: Option<String>,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
) -> Result<(), String> {
    let tracker = tracker.lock().await;
    tracker.set_archive_password(&game_id, password).await
}

/// Get library game ID for a source game
#[tauri::command]
pub fn get_library_game_id(
    source_slug: String,
    source_game_id: String,
) -> String {
    generate_library_game_id(&source_slug, &source_game_id)
}

/// Update (or set for the first time) the cover URL for a library game,
/// then immediately trigger local caching. Safe to call on existing games.
#[tauri::command]
pub async fn update_game_cover_url(
    app_handle: AppHandle,
    game_id: String,
    cover_url: String,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
) -> Result<(), String> {
    // Persist the URL on the game record
    {
        let t = tracker.lock().await;
        t.set_cover_url(&game_id, &cover_url).await?;
    }

    // Cache the image in the background
    let tracker_arc = tracker.inner().clone();
    let app_clone = app_handle.clone();
    let id_clone = game_id.clone();
    let url_clone = cover_url.clone();
    tokio::spawn(async move {
        if let Err(e) = cache_cover_impl(&app_clone, &id_clone, &url_clone, tracker_arc).await {
            log::warn!("[Library] update_game_cover_url caching failed for {}: {}", id_clone, e);
        }
    });

    Ok(())
}

// ========== COVER CACHING ==========

/// Download a game's cover image and save it locally for offline use.
/// Idempotent: skips if the file already exists on disk.
#[tauri::command]
pub async fn cache_game_cover(
    app_handle: AppHandle,
    game_id: String,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
) -> Result<String, String> {
    let game = {
        let t = tracker.lock().await;
        t.get_game(&game_id).await
            .ok_or_else(|| format!("Game not found: {}", game_id))?
    };

    let cover_url = game.cover_url
        .ok_or_else(|| "Game has no cover URL".to_string())?;

    // Already cached and file still exists → just return the path
    if let Some(ref existing) = game.cover_path {
        if std::path::Path::new(existing).exists() {
            return Ok(existing.clone());
        }
    }

    let tracker_arc = tracker.inner().clone();
    cache_cover_impl(&app_handle, &game_id, &cover_url, tracker_arc).await?;

    let t = tracker.lock().await;
    Ok(t.get_game(&game_id).await
        .and_then(|g| g.cover_path)
        .unwrap_or_default())
}

/// Cache covers for all library games that have a URL but no local file yet.
/// Call on app startup so existing library games get offline covers too.
#[tauri::command]
pub async fn cache_missing_covers(
    app_handle: AppHandle,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
) -> Result<(), String> {
    let games = {
        let t = tracker.lock().await;
        t.get_all_games().await
    };

    let tracker_arc = tracker.inner().clone();

    for game in games {
        // Skip if already cached and file exists
        if let Some(ref path) = game.cover_path {
            if std::path::Path::new(path).exists() {
                continue;
            }
        }

        if let Some(ref url) = game.cover_url {
            let app_clone = app_handle.clone();
            let id = game.id.clone();
            let url_clone = url.clone();
            let tracker_clone = tracker_arc.clone();
            tokio::spawn(async move {
                if let Err(e) = cache_cover_impl(&app_clone, &id, &url_clone, tracker_clone).await {
                    log::warn!("[Library] Background cover cache failed for {}: {}", id, e);
                }
            });
        }
    }

    Ok(())
}

// ---- Internal helper ----

async fn cache_cover_impl(
    app: &AppHandle,
    game_id: &str,
    cover_url: &str,
    tracker: Arc<TokioMutex<LibraryTracker>>,
) -> Result<(), String> {
    use crate::utils::create_client;

    // Build the covers directory inside app data
    let app_data = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let covers_dir = app_data.join("covers");
    tokio::fs::create_dir_all(&covers_dir).await
        .map_err(|e| format!("Failed to create covers dir: {}", e))?;

    // Derive extension from URL (default to jpg)
    let lower = cover_url.to_lowercase();
    let ext = if lower.contains(".png") { "png" }
              else if lower.contains(".webp") { "webp" }
              else { "jpg" };

    let cover_path = covers_dir.join(format!("{}.{}", game_id, ext));

    // Download only if file is missing
    if !cover_path.exists() {
        let client = create_client()?;
        let response = client
            .get(cover_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch cover: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Cover fetch returned HTTP {}", response.status()));
        }

        let bytes = response.bytes().await
            .map_err(|e| format!("Failed to read cover bytes: {}", e))?;

        tokio::fs::write(&cover_path, &bytes).await
            .map_err(|e| format!("Failed to write cover file: {}", e))?;
    }

    let path_str = cover_path.to_string_lossy().to_string();
    let t = tracker.lock().await;
    t.set_cover_path(game_id, &path_str).await?;

    log::info!("[Library] Cover cached: {} → {}", game_id, path_str);
    Ok(())
}
