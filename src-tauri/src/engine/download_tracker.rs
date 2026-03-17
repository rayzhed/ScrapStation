use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

/// Download status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Queued,
    Pending,
    Downloading,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Persistent download entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadEntry {
    pub id: String,
    pub game_title: String,
    pub file_name: String,
    pub url: String,
    /// The actual direct download URL (resolved from the original URL)
    /// This is used for resume - the original URL might be a page, not a direct download
    #[serde(default)]
    pub resolved_url: Option<String>,
    /// Cookies needed for authenticated downloads
    #[serde(default)]
    pub cookies: Option<String>,
    pub source_id: String,
    pub host_label: String,
    pub host_color: String,
    pub status: DownloadStatus,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub file_path: Option<String>,
    pub error: Option<String>,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub is_resumable: bool,
}

/// Download control signal
#[derive(Debug, Clone, PartialEq)]
pub enum DownloadSignal {
    Continue,
    Pause,
    Cancel,
}

/// Downloads database stored on disk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DownloadsDb {
    pub downloads: Vec<DownloadEntry>,
    pub download_folder: Option<String>,
}

/// Download tracker with persistence and control
pub struct DownloadTracker {
    app_handle: AppHandle,
    db_path: PathBuf,
    downloads: RwLock<HashMap<String, DownloadEntry>>,
    signals: Arc<Mutex<HashMap<String, DownloadSignal>>>,
}

impl DownloadTracker {
    /// Create a new download tracker
    pub fn new(app_handle: AppHandle) -> Self {
        let db_path = get_downloads_db_path(&app_handle);

        Self {
            app_handle,
            db_path,
            downloads: RwLock::new(HashMap::new()),
            signals: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Load downloads from disk
    pub async fn load(&self) -> Result<(), String> {
        if !self.db_path.exists() {
            log::info!("[DownloadTracker] No existing downloads database");
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&self.db_path)
            .await
            .map_err(|e| format!("Failed to read downloads db: {}", e))?;

        let db: DownloadsDb = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse downloads db: {}", e))?;

        let mut downloads = self.downloads.write().await;
        for entry in db.downloads {
            let mut entry = entry;

            // Completed entries whose file no longer exists on disk are ghosts — drop them.
            if entry.status == DownloadStatus::Completed {
                let file_exists = entry.file_path.as_ref()
                    .map(|p| std::path::Path::new(p).exists())
                    .unwrap_or(false);
                if !file_exists {
                    log::info!("[DownloadTracker] Dropping ghost completed download '{}' (file missing)", entry.id);
                    continue;
                }
            }

            // Reset in-flight statuses on startup.
            match entry.status {
                DownloadStatus::Downloading => entry.status = DownloadStatus::Paused,
                // Queued items lose their position across restarts — treat as paused.
                DownloadStatus::Queued | DownloadStatus::Pending => entry.status = DownloadStatus::Paused,
                _ => {}
            }

            downloads.insert(entry.id.clone(), entry);
        }

        log::info!("[DownloadTracker] Loaded {} downloads from disk", downloads.len());
        Ok(())
    }

    /// Save downloads to disk
    pub async fn save(&self) -> Result<(), String> {
        let downloads = self.downloads.read().await;
        let entries: Vec<DownloadEntry> = downloads.values().cloned().collect();

        let db = DownloadsDb {
            downloads: entries,
            download_folder: Some(get_download_folder(&self.app_handle).to_string_lossy().to_string()),
        };

        let content = serde_json::to_string_pretty(&db)
            .map_err(|e| format!("Failed to serialize downloads: {}", e))?;

        // Ensure parent directory exists
        if let Some(parent) = self.db_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create downloads directory: {}", e))?;
        }

        tokio::fs::write(&self.db_path, content)
            .await
            .map_err(|e| format!("Failed to write downloads db: {}", e))?;

        Ok(())
    }

    /// Add a new download
    pub async fn add_download(&self, entry: DownloadEntry) -> Result<(), String> {
        {
            let mut downloads = self.downloads.write().await;
            downloads.insert(entry.id.clone(), entry.clone());
        }

        // Initialize signal
        {
            let mut signals = self.signals.lock().await;
            signals.insert(entry.id.clone(), DownloadSignal::Continue);
        }

        self.save().await?;
        self.emit_downloads_update().await;
        Ok(())
    }

    /// Update download progress
    pub async fn update_progress(
        &self,
        id: &str,
        downloaded_bytes: u64,
        total_bytes: u64,
    ) -> Result<(), String> {
        {
            let mut downloads = self.downloads.write().await;
            if let Some(entry) = downloads.get_mut(id) {
                // Never override terminal states with a progress update.
                // This prevents a fire-and-forget tokio::spawn from racing
                // with cancel_download and restoring a "Downloading" status.
                match entry.status {
                    DownloadStatus::Cancelled
                    | DownloadStatus::Failed
                    | DownloadStatus::Completed => return Ok(()),
                    _ => {}
                }
                entry.downloaded_bytes = downloaded_bytes;
                entry.total_bytes = total_bytes;
                entry.status = DownloadStatus::Downloading;
            }
        }

        // Don't save on every progress update - too slow
        Ok(())
    }

    /// Update download status
    pub async fn update_status(&self, id: &str, status: DownloadStatus) -> Result<(), String> {
        {
            let mut downloads = self.downloads.write().await;
            if let Some(entry) = downloads.get_mut(id) {
                entry.status = status;
                if entry.status == DownloadStatus::Completed {
                    entry.completed_at = Some(std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64);
                }
            }
        }

        self.save().await?;
        self.emit_downloads_update().await;
        Ok(())
    }

    /// Complete a download
    pub async fn complete_download(&self, id: &str, file_path: &str, file_size: u64) -> Result<(), String> {
        let should_emit = {
            let mut downloads = self.downloads.write().await;
            if let Some(entry) = downloads.get_mut(id) {
                // Don't complete a cancelled download — e.g. a WebView browser download
                // that finishes after the user already cancelled from the UI.
                if entry.status == DownloadStatus::Cancelled {
                    return Ok(());
                }
                entry.status = DownloadStatus::Completed;
                entry.file_path = Some(file_path.to_string());
                entry.downloaded_bytes = file_size;
                entry.total_bytes = file_size;
                entry.completed_at = Some(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64);
                true
            } else {
                false
            }
        };

        if should_emit {
            self.save().await?;
            self.emit_downloads_update().await;
        }
        Ok(())
    }

    /// Fail a download
    pub async fn fail_download(&self, id: &str, error: &str) -> Result<(), String> {
        {
            let mut downloads = self.downloads.write().await;
            if let Some(entry) = downloads.get_mut(id) {
                entry.status = DownloadStatus::Failed;
                entry.error = Some(error.to_string());
            }
        }

        self.save().await?;
        self.emit_downloads_update().await;
        Ok(())
    }

    /// Pause a download
    pub async fn pause_download(&self, id: &str) -> Result<(), String> {
        log::info!("[DownloadTracker] Pausing download: {}", id);

        // Set signal to pause
        {
            let mut signals = self.signals.lock().await;
            signals.insert(id.to_string(), DownloadSignal::Pause);
        }

        // Update status
        {
            let mut downloads = self.downloads.write().await;
            if let Some(entry) = downloads.get_mut(id) {
                if entry.status == DownloadStatus::Downloading {
                    entry.status = DownloadStatus::Paused;
                }
            }
        }

        self.save().await?;
        self.emit_downloads_update().await;
        Ok(())
    }

    /// Resume a download (sets signal, actual resume handled by download command)
    pub async fn resume_download(&self, id: &str) -> Result<DownloadEntry, String> {
        log::info!("[DownloadTracker] Resuming download: {}", id);

        let entry = {
            let mut downloads = self.downloads.write().await;
            if let Some(entry) = downloads.get_mut(id) {
                if entry.status == DownloadStatus::Paused || entry.status == DownloadStatus::Failed {
                    entry.status = DownloadStatus::Pending;
                    entry.error = None;
                }
                entry.clone()
            } else {
                return Err(format!("Download not found: {}", id));
            }
        };

        // Set signal to continue
        {
            let mut signals = self.signals.lock().await;
            signals.insert(id.to_string(), DownloadSignal::Continue);
        }

        self.save().await?;
        self.emit_downloads_update().await;
        Ok(entry)
    }

    /// Cancel a download
    pub async fn cancel_download(&self, id: &str) -> Result<(), String> {
        log::info!("[DownloadTracker] Cancelling download: {}", id);

        // Set signal to cancel
        {
            let mut signals = self.signals.lock().await;
            signals.insert(id.to_string(), DownloadSignal::Cancel);
        }

        // Update status and grab the partial file path (if any) so we can delete it
        let partial_file = {
            let mut downloads = self.downloads.write().await;
            if let Some(entry) = downloads.get_mut(id) {
                entry.status = DownloadStatus::Cancelled;
                entry.file_path.clone()
            } else {
                None
            }
        };

        // Delete the partial file to free disk space.
        // For streaming downloads the download loop also does this, so a double-
        // delete is possible — ignore any error from the second attempt.
        if let Some(path) = partial_file {
            if !path.is_empty() {
                let _ = std::fs::remove_file(&path);
                log::info!("[DownloadTracker] Deleted partial file on cancel: {}", path);
            }
        }

        self.save().await?;
        self.emit_downloads_update().await;
        Ok(())
    }

    /// Get a snapshot of the entries matching the given IDs.
    /// Used by auto-extract to check whether all parts of a multi-file download
    /// have completed before triggering extraction.
    pub async fn get_entries_by_ids(&self, ids: &[String]) -> Vec<DownloadEntry> {
        let downloads = self.downloads.read().await;
        ids.iter()
            .filter_map(|id| downloads.get(id).cloned())
            .collect()
    }

    /// Remove a download from the list
    pub async fn remove_download(&self, id: &str) -> Result<(), String> {
        log::info!("[DownloadTracker] Removing download: {}", id);

        {
            let mut downloads = self.downloads.write().await;
            downloads.remove(id);
        }

        {
            let mut signals = self.signals.lock().await;
            signals.remove(id);
        }

        self.save().await?;
        self.emit_downloads_update().await;
        Ok(())
    }

    /// Clear all completed/failed/cancelled downloads
    pub async fn clear_finished(&self) -> Result<(), String> {
        log::info!("[DownloadTracker] Clearing finished downloads");

        {
            let mut downloads = self.downloads.write().await;
            downloads.retain(|_, entry| {
                matches!(entry.status, DownloadStatus::Queued | DownloadStatus::Downloading | DownloadStatus::Pending | DownloadStatus::Paused)
            });
        }

        self.save().await?;
        self.emit_downloads_update().await;
        Ok(())
    }

    /// Get all downloads
    pub async fn get_all_downloads(&self) -> Vec<DownloadEntry> {
        let downloads = self.downloads.read().await;
        downloads.values().cloned().collect()
    }

    /// Get a download by ID
    pub async fn get_download(&self, id: &str) -> Option<DownloadEntry> {
        let downloads = self.downloads.read().await;
        downloads.get(id).cloned()
    }

    /// Check download signal (for pause/cancel during download)
    pub async fn check_signal(&self, id: &str) -> DownloadSignal {
        let signals = self.signals.lock().await;
        signals.get(id).cloned().unwrap_or(DownloadSignal::Continue)
    }

    /// Get the signals map for sharing with download tasks
    pub fn get_signals(&self) -> Arc<Mutex<HashMap<String, DownloadSignal>>> {
        self.signals.clone()
    }

    /// Mark a download as non-resumable (e.g., WebView downloads)
    pub async fn mark_non_resumable(&self, id: &str) -> Result<(), String> {
        {
            let mut downloads = self.downloads.write().await;
            if let Some(entry) = downloads.get_mut(id) {
                entry.is_resumable = false;
            }
        }
        self.save().await?;
        self.emit_downloads_update().await;
        Ok(())
    }

    /// Mark a download as resumable (e.g., when using reqwest with Range support)
    pub async fn mark_resumable(&self, id: &str) -> Result<(), String> {
        {
            let mut downloads = self.downloads.write().await;
            if let Some(entry) = downloads.get_mut(id) {
                entry.is_resumable = true;
            }
        }
        self.save().await?;
        self.emit_downloads_update().await;
        Ok(())
    }

    /// Update the resolved (direct) download URL - used for resume
    pub async fn update_resolved_url(&self, id: &str, resolved_url: &str) -> Result<(), String> {
        {
            let mut downloads = self.downloads.write().await;
            if let Some(entry) = downloads.get_mut(id) {
                entry.resolved_url = Some(resolved_url.to_string());
                log::info!("[DownloadTracker] Updated resolved URL for {}: {}", id, resolved_url);
            }
        }
        self.save().await?;
        Ok(())
    }

    /// Update cookies for a download (needed for authenticated resume)
    pub async fn update_cookies(&self, id: &str, cookies: &str) -> Result<(), String> {
        {
            let mut downloads = self.downloads.write().await;
            if let Some(entry) = downloads.get_mut(id) {
                entry.cookies = Some(cookies.to_string());
                log::info!("[DownloadTracker] Saved cookies for {}", id);
            }
        }
        self.save().await?;
        Ok(())
    }

    /// Update the file path for a download (called when download starts)
    pub async fn update_file_path(&self, id: &str, file_path: &str) -> Result<(), String> {
        {
            let mut downloads = self.downloads.write().await;
            if let Some(entry) = downloads.get_mut(id) {
                entry.file_path = Some(file_path.to_string());
                log::info!("[DownloadTracker] Updated file path for {}: {}", id, file_path);
            }
        }
        self.save().await?;
        self.emit_downloads_update().await;
        Ok(())
    }

    /// Save download state when pausing (progress, file path, etc.)
    pub async fn pause_download_with_progress(&self, id: &str, downloaded_bytes: u64, total_bytes: u64) -> Result<(), String> {
        {
            let mut downloads = self.downloads.write().await;
            if let Some(entry) = downloads.get_mut(id) {
                entry.status = DownloadStatus::Paused;
                entry.downloaded_bytes = downloaded_bytes;
                entry.total_bytes = total_bytes;
                log::info!("[DownloadTracker] Paused download {}: {} / {} bytes", id, downloaded_bytes, total_bytes);
            }
        }
        self.save().await?;
        self.emit_downloads_update().await;
        Ok(())
    }

    /// Update the filename of a download
    pub async fn update_filename(&self, id: &str, filename: &str) -> Result<(), String> {
        {
            let mut downloads = self.downloads.write().await;
            if let Some(entry) = downloads.get_mut(id) {
                entry.file_name = filename.to_string();
            }
        }
        self.save().await?;

        // Emit specific filename update event
        if let Err(e) = self.app_handle.emit("download-filename-updated", serde_json::json!({
            "id": id,
            "file_name": filename
        })) {
            log::warn!("[DownloadTracker] Failed to emit filename update: {}", e);
        }

        self.emit_downloads_update().await;
        Ok(())
    }

    /// Emit downloads update event to frontend
    async fn emit_downloads_update(&self) {
        let downloads = self.get_all_downloads().await;
        if let Err(e) = self.app_handle.emit("downloads-updated", &downloads) {
            log::warn!("[DownloadTracker] Failed to emit downloads update: {}", e);
        }
    }
}

/// Get the path to the downloads database file
fn get_downloads_db_path(app_handle: &AppHandle) -> PathBuf {
    let app_data = app_handle.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    app_data.join("downloads.json")
}

/// Get the download folder path
pub fn get_download_folder(app_handle: &AppHandle) -> PathBuf {
    let app_data = app_handle.path().app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let downloads_dir = app_data.join("Downloads");

    // Create the directory if it doesn't exist
    if !downloads_dir.exists() {
        let _ = std::fs::create_dir_all(&downloads_dir);
    }

    downloads_dir
}

/// Generate a unique download ID
pub fn generate_download_id() -> String {
    format!("dl_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        fastrand::u32(..)
    )
}
