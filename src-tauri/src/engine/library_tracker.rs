use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

/// Library game status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LibraryGameStatus {
    Downloading,
    Extracting,
    Ready,
    Corrupted,
}

/// Executable type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExeType {
    Main,
    Launcher,
    Tool,
    Installer,
    Redistributable,
    Unknown,
}

/// Game executable information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameExecutable {
    pub path: String,
    pub name: String,
    pub score: f32,
    pub exe_type: ExeType,
}

/// Library game entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryGame {
    pub id: String,
    pub source_slug: String,
    pub source_game_id: String,
    pub title: String,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub cover_path: Option<String>,
    pub install_path: String,
    #[serde(default)]
    pub install_size: u64,
    pub status: LibraryGameStatus,
    pub installed_at: u64,
    #[serde(default)]
    pub last_played: Option<u64>,
    #[serde(default)]
    pub total_playtime: u64,
    #[serde(default)]
    pub executables: Vec<GameExecutable>,
    #[serde(default)]
    pub primary_exe: Option<String>,
    #[serde(default)]
    pub archive_password: Option<String>,
    #[serde(default)]
    pub download_ids: Vec<String>,
}

/// Library database stored on disk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LibraryDb {
    pub games: Vec<LibraryGame>,
    pub library_folder: Option<String>,
}

/// Library tracker with persistence
pub struct LibraryTracker {
    app_handle: AppHandle,
    db_path: PathBuf,
    games: RwLock<HashMap<String, LibraryGame>>,
}

impl LibraryTracker {
    /// Create a new library tracker
    pub fn new(app_handle: AppHandle) -> Self {
        let db_path = get_library_db_path(&app_handle);

        Self {
            app_handle,
            db_path,
            games: RwLock::new(HashMap::new()),
        }
    }

    /// Load library from disk
    pub async fn load(&self) -> Result<(), String> {
        if !self.db_path.exists() {
            log::info!("[LibraryTracker] No existing library database");
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&self.db_path)
            .await
            .map_err(|e| format!("Failed to read library db: {}", e))?;

        let db: LibraryDb = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse library db: {}", e))?;

        let mut games = self.games.write().await;
        for game in db.games {
            games.insert(game.id.clone(), game);
        }

        log::info!("[LibraryTracker] Loaded {} games from disk", games.len());
        Ok(())
    }

    /// Save library to disk
    pub async fn save(&self) -> Result<(), String> {
        let games = self.games.read().await;
        let entries: Vec<LibraryGame> = games.values().cloned().collect();

        let db = LibraryDb {
            games: entries,
            library_folder: Some(get_library_folder(&self.app_handle).to_string_lossy().to_string()),
        };

        let content = serde_json::to_string_pretty(&db)
            .map_err(|e| format!("Failed to serialize library: {}", e))?;

        // Ensure parent directory exists
        if let Some(parent) = self.db_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create library directory: {}", e))?;
        }

        tokio::fs::write(&self.db_path, content)
            .await
            .map_err(|e| format!("Failed to write library db: {}", e))?;

        Ok(())
    }

    /// Add a new game to the library
    pub async fn add_game(&self, game: LibraryGame) -> Result<(), String> {
        {
            let mut games = self.games.write().await;
            games.insert(game.id.clone(), game);
        }

        self.save().await?;
        self.emit_library_update().await;
        Ok(())
    }

    /// Update an existing game
    pub async fn update_game(&self, game: LibraryGame) -> Result<(), String> {
        {
            let mut games = self.games.write().await;
            if !games.contains_key(&game.id) {
                return Err(format!("Game not found: {}", game.id));
            }
            games.insert(game.id.clone(), game);
        }

        self.save().await?;
        self.emit_library_update().await;
        Ok(())
    }

    /// Remove a game from the library
    pub async fn remove_game(&self, id: &str) -> Result<Option<LibraryGame>, String> {
        let removed = {
            let mut games = self.games.write().await;
            games.remove(id)
        };

        self.save().await?;
        self.emit_library_update().await;
        Ok(removed)
    }

    /// Get a game by ID
    pub async fn get_game(&self, id: &str) -> Option<LibraryGame> {
        let games = self.games.read().await;
        games.get(id).cloned()
    }

    /// Get all games
    pub async fn get_all_games(&self) -> Vec<LibraryGame> {
        let games = self.games.read().await;
        games.values().cloned().collect()
    }

    /// Find game by source and source_game_id
    pub async fn find_by_source(&self, source_slug: &str, source_game_id: &str) -> Option<LibraryGame> {
        let games = self.games.read().await;
        games.values()
            .find(|g| g.source_slug == source_slug && g.source_game_id == source_game_id)
            .cloned()
    }

    /// Find game by a linked download ID
    pub async fn find_by_download_id(&self, download_id: &str) -> Option<LibraryGame> {
        let games = self.games.read().await;
        games.values()
            .find(|g| g.download_ids.contains(&download_id.to_string()))
            .cloned()
    }

    /// Update game status
    pub async fn update_status(&self, id: &str, status: LibraryGameStatus) -> Result<(), String> {
        {
            let mut games = self.games.write().await;
            if let Some(game) = games.get_mut(id) {
                game.status = status;
            } else {
                return Err(format!("Game not found: {}", id));
            }
        }

        self.save().await?;
        self.emit_library_update().await;
        Ok(())
    }

    /// Set the primary executable for a game
    pub async fn set_primary_exe(&self, id: &str, exe_path: &str) -> Result<(), String> {
        {
            let mut games = self.games.write().await;
            if let Some(game) = games.get_mut(id) {
                game.primary_exe = Some(exe_path.to_string());
            } else {
                return Err(format!("Game not found: {}", id));
            }
        }

        self.save().await?;
        self.emit_library_update().await;
        Ok(())
    }

    /// Update executables list
    pub async fn set_executables(&self, id: &str, executables: Vec<GameExecutable>) -> Result<(), String> {
        {
            let mut games = self.games.write().await;
            if let Some(game) = games.get_mut(id) {
                // Auto-select primary exe if not set
                if game.primary_exe.is_none() && !executables.is_empty() {
                    // Find the highest scoring non-installer executable
                    if let Some(best) = executables.iter()
                        .filter(|e| e.exe_type != ExeType::Installer && e.exe_type != ExeType::Redistributable)
                        .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
                    {
                        game.primary_exe = Some(best.path.clone());
                    }
                }
                game.executables = executables;
            } else {
                return Err(format!("Game not found: {}", id));
            }
        }

        self.save().await?;
        self.emit_library_update().await;
        Ok(())
    }

    /// Update install size
    pub async fn set_install_size(&self, id: &str, size: u64) -> Result<(), String> {
        {
            let mut games = self.games.write().await;
            if let Some(game) = games.get_mut(id) {
                game.install_size = size;
            }
        }

        self.save().await?;
        Ok(())
    }

    /// Update playtime
    pub async fn update_playtime(&self, id: &str, additional_seconds: u64) -> Result<(), String> {
        {
            let mut games = self.games.write().await;
            if let Some(game) = games.get_mut(id) {
                game.total_playtime += additional_seconds;
                game.last_played = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                );
            }
        }

        self.save().await?;
        self.emit_library_update().await;
        Ok(())
    }

    /// Add a download ID to a game
    pub async fn add_download_id(&self, id: &str, download_id: &str) -> Result<(), String> {
        {
            let mut games = self.games.write().await;
            if let Some(game) = games.get_mut(id) {
                if !game.download_ids.contains(&download_id.to_string()) {
                    game.download_ids.push(download_id.to_string());
                }
            }
        }

        self.save().await?;
        Ok(())
    }

    /// Set cover path (after caching)
    pub async fn set_cover_path(&self, id: &str, path: &str) -> Result<(), String> {
        {
            let mut games = self.games.write().await;
            if let Some(game) = games.get_mut(id) {
                game.cover_path = Some(path.to_string());
            }
        }

        self.save().await?;
        self.emit_library_update().await;
        Ok(())
    }

    /// Set cover URL (and clear stale cover_path so it gets re-downloaded)
    pub async fn set_cover_url(&self, id: &str, url: &str) -> Result<(), String> {
        {
            let mut games = self.games.write().await;
            if let Some(game) = games.get_mut(id) {
                game.cover_url = Some(url.to_string());
                // Clear cached path so it gets re-downloaded
                game.cover_path = None;
            } else {
                return Err(format!("Game not found: {}", id));
            }
        }

        self.save().await?;
        Ok(())
    }

    /// Set archive password (for existing games that need password update)
    pub async fn set_archive_password(&self, id: &str, password: Option<String>) -> Result<(), String> {
        {
            let mut games = self.games.write().await;
            if let Some(game) = games.get_mut(id) {
                game.archive_password = password;
            } else {
                return Err(format!("Game not found: {}", id));
            }
        }

        self.save().await?;
        log::info!("[LibraryTracker] Updated archive password for game: {}", id);
        Ok(())
    }

    /// Emit library update event to frontend
    pub async fn emit_library_update(&self) {
        let games = self.get_all_games().await;
        if let Err(e) = self.app_handle.emit("library-updated", &games) {
            log::warn!("[LibraryTracker] Failed to emit library update: {}", e);
        }
    }
}

/// Get the path to the library database file
fn get_library_db_path(app_handle: &AppHandle) -> PathBuf {
    let app_data = app_handle.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    app_data.join("library.json")
}

/// Get the library folder path (where games are installed)
pub fn get_library_folder(app_handle: &AppHandle) -> PathBuf {
    let app_data = app_handle.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let library_dir = app_data.join("Library");

    // Create the directory if it doesn't exist
    if !library_dir.exists() {
        let _ = std::fs::create_dir_all(&library_dir);
    }

    library_dir
}

/// Generate a unique library game ID from source info
pub fn generate_library_game_id(source_slug: &str, source_game_id: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    source_game_id.hash(&mut hasher);
    let hash = hasher.finish();

    format!("{}_{:x}", source_slug, hash)
}

/// Generate a slug from a game title (for folder names)
/// Only uses ASCII alphanumeric characters to avoid path issues on Windows
pub fn generate_game_slug(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| {
            // Only allow ASCII alphanumeric characters to avoid path issues
            if c.is_ascii_alphanumeric() {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Get the folder path for a specific game
pub fn get_game_folder(app_handle: &AppHandle, game_slug: &str) -> PathBuf {
    get_library_folder(app_handle).join(game_slug)
}

/// Get current timestamp in milliseconds
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
