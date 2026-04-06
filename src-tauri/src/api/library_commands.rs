use crate::engine::library_tracker::{
    LibraryTracker, LibraryGame, LibraryGameStatus, GameExecutable, ExeType,
    get_library_folder, resolve_install_path,
    generate_library_game_id, generate_game_slug, current_timestamp
};
use crate::engine::archive_extractor::{ArchiveExtractor, delete_archives};
use crate::engine::executable_detector::ExecutableDetector;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State, Emitter};
use tauri_plugin_shell::ShellExt;
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
    // Store absolute path so the game is always found regardless of data root changes
    let install_path = get_library_folder(&app_handle)
        .join(format!("{}/game", game_slug))
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
    app_handle: AppHandle,
) -> Result<(), String> {
    let tracker = tracker.lock().await;

    // Get game info before removing
    let game = tracker.get_game(&id).await;

    // Remove from database
    tracker.remove_game(&id).await?;

    if let Some(g) = game {
        // Always delete the cached cover — it's just a local cache file
        if let Some(cover_path) = &g.cover_path {
            let path = PathBuf::from(cover_path);
            if path.exists() {
                if let Err(e) = tokio::fs::remove_file(&path).await {
                    log::warn!("[Library] Failed to delete cover file: {}", e);
                } else {
                    log::info!("[Library] Deleted cover: {:?}", path);
                }
            }
        }

        // Delete game files if requested
        if delete_files {
            let install_path = resolve_install_path(&g, &app_handle);
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

    // Guard against concurrent extractions for the same game
    if matches!(game.status, LibraryGameStatus::Extracting) {
        log::warn!("[Library] Extraction already in progress for {} — ignoring duplicate call.", game_id);
        return Ok(());
    }

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

    // Determine destination (resolve relative install_path to absolute)
    let destination = resolve_install_path(&game, &app_handle);

    // ── Pre-extraction disk space check ──────────────────────────────────────
    {
        let archive_bytes: u64 = paths.iter()
            .filter_map(|p| std::fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();

        if archive_bytes > 0 {
            let estimated_extract: u64 = (archive_bytes as f64 * 2.5) as u64;
            let available = crate::api::download_commands::available_bytes_at(&destination);

            if available > 0 && available < estimated_extract {
                let err = format!(
                    "disk_space: Not enough disk space to extract. Needs ~{} MB but only {} MB available.",
                    estimated_extract / 1_048_576,
                    available / 1_048_576
                );
                log::warn!("[Library] {}", err);
                {
                    let tracker = library_tracker.lock().await;
                    tracker.update_status(&game_id, LibraryGameStatus::Corrupted).await?;
                }
                let _ = app_handle.emit("extraction-error", serde_json::json!({
                    "gameId": game_id,
                    "error": err
                }));
                return Err(err);
            }
        }
    }

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

            if extraction_result.files_extracted == 0 {
                log::error!("[Library] Extraction reported success but 0 files were extracted.");
                let _ = library_tracker.lock().await.update_status(&game_id, LibraryGameStatus::Corrupted).await;
                let _ = app_handle.emit("extraction-error", serde_json::json!({
                    "gameId": game_id,
                    "error": "disk_space: Not enough disk space — no files were extracted."
                }));
                return Err("disk_space: Not enough disk space — no files were extracted.".to_string());
            }

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
            // Tag disk-full errors so the frontend can show a specific message
            let err_str = e.to_string();
            let tagged = if is_disk_full_error(&err_str) {
                format!("disk_space: {}", err_str)
            } else {
                err_str
            };
            log::error!("[Library] Extraction failed: {}", tagged);

            // Mark as corrupted so the Extract button can still find and retry it
            let _ = library_tracker.lock().await.update_status(&game_id, LibraryGameStatus::Corrupted).await;

            let _ = app_handle.emit("extraction-error", serde_json::json!({
                "gameId": game_id,
                "error": tagged
            }));

            Err(tagged)
        }
    }
}

// ========== GAME LAUNCH ==========

/// Launch a game
#[tauri::command]
pub async fn launch_game(
    id: String,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let game = {
        let tracker = tracker.lock().await;
        tracker.get_game(&id).await
            .ok_or_else(|| format!("Game not found: {}", id))?
    };

    if game.status != LibraryGameStatus::Ready {
        return Err("Game is not ready to launch".to_string());
    }

    let exe_path = game.primary_exe.clone()
        .ok_or_else(|| "No executable selected".to_string())?;

    let full_exe_path = resolve_install_path(&game, &app_handle).join(&exe_path);

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
    app_handle: AppHandle,
) -> Result<(), String> {
    let game = {
        let tracker = tracker.lock().await;
        tracker.get_game(&id).await
            .ok_or_else(|| format!("Game not found: {}", id))?
    };

    let folder = resolve_install_path(&game, &app_handle);
    log::info!("[Library] open_game_folder: stored='{}' resolved='{}'", game.install_path, folder.display());

    if !folder.exists() {
        return Err(format!(
            "Game folder not found: {}. Try running Settings → Recovery → Fix Broken Paths.",
            folder.display()
        ));
    }

    open_folder_in_explorer(&app_handle, &folder)
}

/// Get library folder path
#[tauri::command]
pub async fn get_library_folder_path(
    app_handle: AppHandle,
) -> Result<String, String> {
    Ok(get_library_folder(&app_handle).to_string_lossy().to_string())
}

/// Open the overall library folder in the system file explorer
#[tauri::command]
pub async fn open_library_folder(app_handle: AppHandle) -> Result<(), String> {
    let folder = get_library_folder(&app_handle);
    open_folder_in_explorer(&app_handle, &folder)
}

/// Move a game's install folder to a new parent directory chosen by the user.
/// The game subfolder name is preserved; only its parent changes.
#[tauri::command]
pub async fn move_game(
    game_id: String,
    target_dir: String,
    tracker: tauri::State<'_, Arc<tokio::sync::Mutex<LibraryTracker>>>,
    app_handle: AppHandle,
) -> Result<String, String> {
    let tracker_arc = tracker.inner().clone();
    let t = tracker_arc.lock().await;

    let game = t.get_game(&game_id).await
        .ok_or_else(|| format!("Game not found: {}", game_id))?;

    // install_path is absolute, e.g. C:\...\Library\slug\game
    let install_path = resolve_install_path(&game, &app_handle);

    // game_root = C:\...\Library\slug  (the folder we actually move)
    let game_root = install_path.parent()
        .ok_or("Could not determine game root folder")?;

    let folder_name = game_root.file_name()
        .ok_or("Could not determine game folder name")?;

    let game_subfolder = install_path.file_name()
        .ok_or("Could not determine game subfolder name")?;

    if !game_root.exists() {
        return Err(format!("Source folder not found: {}", game_root.display()));
    }

    let new_game_root = PathBuf::from(&target_dir).join(folder_name);
    if new_game_root.exists() {
        return Err(format!("Destination already exists: {}", new_game_root.display()));
    }

    // Same-drive move (fast) — fall back to copy+delete for cross-drive
    if std::fs::rename(game_root, &new_game_root).is_err() {
        crate::copy_dir_recursive(game_root, &new_game_root)
            .map_err(|e| format!("Failed to copy game files: {}", e))?;
        std::fs::remove_dir_all(game_root)
            .map_err(|e| format!("Failed to remove old files after copy: {}", e))?;
    }

    let new_install_path = new_game_root.join(game_subfolder)
        .to_string_lossy()
        .to_string();

    t.update_install_path(&game_id, &new_install_path).await?;

    log::info!("[Library] Moved game '{}' to {}", game_id, new_install_path);
    Ok(new_install_path)
}

fn open_folder_in_explorer(app_handle: &AppHandle, path: &std::path::Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err(format!("Resolved path is not absolute: {}", path.display()));
    }
    let path_str = path.to_string_lossy().to_string();
    log::info!("[Library] Opening folder: {}", path_str);
    app_handle.shell().open(&path_str, None)
        .map_err(|e| format!("Failed to open folder: {}", e))
}

/// Rescan executables for a game
#[tauri::command]
pub async fn rescan_game_executables(
    id: String,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
    app_handle: AppHandle,
) -> Result<Vec<GameExecutable>, String> {
    let game = {
        let tracker = tracker.lock().await;
        tracker.get_game(&id).await
            .ok_or_else(|| format!("Game not found: {}", id))?
    };

    let destination = resolve_install_path(&game, &app_handle);
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

// ========== LIBRARY REPAIR ==========

/// Scan the Library folder and repair library.json:
///   - Add game folders that are not tracked at all
///   - Update tracked games whose cover or executables are missing
/// Returns the total number of entries added or updated.
#[tauri::command]
pub async fn repair_library(
    app_handle: AppHandle,
    tracker: State<'_, Arc<TokioMutex<LibraryTracker>>>,
) -> Result<usize, String> {
    let covers_dir = app_handle.path().app_data_dir()
        .map(|d| d.join("covers"))
        .ok();

    // Snapshot of current library (resolved absolute install_path → game)
    let existing: HashMap<String, LibraryGame> = {
        let t = tracker.lock().await;
        t.get_all_games().await.into_iter().map(|g| {
            let abs = resolve_install_path(&g, &app_handle).to_string_lossy().to_string();
            (abs, g)
        }).collect()
    };

    let mut fixed = 0usize;
    // Track which game IDs were already handled in phase 1
    let mut handled_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // ── Phase 1: walk all known Library/ folders ─────────────────────────────
    let library_folders = crate::api::settings_commands::all_library_paths(&app_handle);
    for library_folder in &library_folders {
    if library_folder.exists() {
        let mut dir = tokio::fs::read_dir(&library_folder)
            .await
            .map_err(|e| format!("Failed to read library folder: {}", e))?;

        while let Ok(Some(entry)) = dir.next_entry().await {
            let folder_path = entry.path();
            if !folder_path.is_dir() { continue; }

            let game_path = folder_path.join("game");
            if !game_path.is_dir() { continue; }

            let game_path_str = game_path.to_string_lossy().to_string();
            let folder_name = folder_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            if let Some(game) = existing.get(&game_path_str) {
                // ── Already tracked: patch what's missing ────────────────
                handled_ids.insert(game.id.clone());
                let mut updated = false;

                // Cover: missing or pointing to a file that no longer exists
                let cover_missing = game.cover_path.as_ref()
                    .map(|p| !std::path::Path::new(p).exists())
                    .unwrap_or(true);

                if cover_missing {
                    if let Some(cover) = find_cover_by_slug(&covers_dir, &folder_name) {
                        let t = tracker.lock().await;
                        t.set_cover_path(&game.id, &cover).await?;
                        updated = true;
                        log::info!("[Library] Repair: restored cover for '{}'", game.title);
                    }
                }

                // Executables: empty list or no primary selected
                if game.executables.is_empty() || game.primary_exe.is_none() {
                    let exes = ExecutableDetector::detect_executables(&game_path, &game.title);
                    if !exes.is_empty() {
                        let t = tracker.lock().await;
                        t.set_executables(&game.id, exes).await?;
                        updated = true;
                        log::info!("[Library] Repair: rescanned executables for '{}'", game.title);
                    }
                }

                if updated { fixed += 1; }
            } else {
                // ── Not tracked: add as a new recovered entry ─────────────
                let title: String = folder_name
                    .split('-')
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                let id = generate_library_game_id("recovered", &folder_name);
                let cover_path = find_cover_by_slug(&covers_dir, &folder_name);
                let executables = ExecutableDetector::detect_executables(&game_path, &title);
                let install_size = ExecutableDetector::calculate_directory_size(&game_path);
                let primary_exe = executables.iter()
                    .filter(|e| e.exe_type != ExeType::Installer && e.exe_type != ExeType::Redistributable)
                    .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|e| e.path.clone());

                let game = LibraryGame {
                    id: id.clone(),
                    source_slug: "recovered".to_string(),
                    source_game_id: folder_name.clone(),
                    title,
                    cover_url: None,
                    cover_path,
                    // Store absolute path so the game is found regardless of which
                    // library root is currently active.
                    install_path: game_path_str.clone(),
                    install_size,
                    status: LibraryGameStatus::Ready,
                    installed_at: current_timestamp(),
                    last_played: None,
                    total_playtime: 0,
                    executables,
                    primary_exe,
                    archive_password: None,
                    download_ids: vec![],
                };

                {
                    let t = tracker.lock().await;
                    t.add_game(game).await?;
                }
                fixed += 1;
                log::info!("[Library] Repair: added missing game '{}' from {:?}", id, library_folder);
            }
        }
    } // end exists check
    } // end library_folder loop

    // ── Phase 2: fix covers for tracked games not found in Library/ ──────────
    // (e.g. games installed to a custom path, or Library/ didn't exist)
    for (_, game) in &existing {
        if handled_ids.contains(&game.id) { continue; }

        let cover_missing = game.cover_path.as_ref()
            .map(|p| !std::path::Path::new(p).exists())
            .unwrap_or(true);

        if cover_missing {
            let slug = generate_game_slug(&game.title);
            if let Some(cover) = find_cover_by_slug(&covers_dir, &slug) {
                let t = tracker.lock().await;
                t.set_cover_path(&game.id, &cover).await?;
                fixed += 1;
                log::info!("[Library] Repair: restored cover for '{}' (phase 2)", game.title);
            }
        }
    }

    log::info!("[Library] Repair complete: {} item(s) added/updated", fixed);
    Ok(fixed)
}

/// Find the first matching cover image for a slug in the covers directory.
fn find_cover_by_slug(covers_dir: &Option<std::path::PathBuf>, slug: &str) -> Option<String> {
    let dir = covers_dir.as_ref()?;
    for ext in &["jpg", "png", "webp"] {
        let candidate = dir.join(format!("{}.{}", slug, ext));
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
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

    // Use the game's title slug as the filename so covers are human-readable and
    // can be matched back to their folder during a library repair.
    let cover_filename = {
        let t = tracker.lock().await;
        if let Some(game) = t.get_game(game_id).await {
            generate_game_slug(&game.title)
        } else {
            game_id.to_string() // fallback: game not found yet
        }
    };

    // Derive extension from URL (default to jpg)
    let lower = cover_url.to_lowercase();
    let ext = if lower.contains(".png") { "png" }
              else if lower.contains(".webp") { "webp" }
              else { "jpg" };

    let cover_path = covers_dir.join(format!("{}.{}", cover_filename, ext));

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

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Returns true when the error string looks like a disk-full / out-of-space error.
fn is_disk_full_error(err: &str) -> bool {
    let s = err.to_lowercase();
    s.contains("disk is full")
        || s.contains("no space left")
        || s.contains("not enough space")
        || s.contains("storage full")
        || s.contains("insufficient disk space")
        || s.contains("there is not enough space")
}
