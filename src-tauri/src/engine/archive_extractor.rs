use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

/// Archive format types
#[derive(Debug, Clone, PartialEq)]
pub enum ArchiveFormat {
    SevenZip,
    Rar,
    Zip,
    Unknown,
}

/// Multi-part archive pattern
#[derive(Debug, Clone)]
pub enum MultiPartPattern {
    RarPart,      // .part1.rar, .part2.rar
    RarVolume,    // .rar, .r00, .r01
    SevenZipPart, // .7z.001, .7z.002
    ZipSplit,     // .zip.001, .z01
    None,
}

/// Extraction progress event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionProgress {
    pub game_id: String,
    pub current_file: String,
    pub files_done: u32,
    pub files_total: u32,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub current_archive: u32,
    pub total_archives: u32,
}

/// Extraction result
#[derive(Debug, Clone, Serialize)]
pub struct ExtractionResult {
    pub success: bool,
    pub files_extracted: u32,
    pub total_size: u64,
    pub destination: String,
    pub error: Option<String>,
}

/// Extraction error types
#[derive(Debug)]
pub enum ExtractionError {
    UnsupportedFormat(String),
    PasswordRequired,
    WrongPassword,
    CorruptedArchive(String),
    IoError(String),
    MultiPartMissing(String),
}

impl std::fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractionError::UnsupportedFormat(s) => write!(f, "Unsupported format: {}", s),
            ExtractionError::PasswordRequired => write!(f, "Password required"),
            ExtractionError::WrongPassword => write!(f, "Wrong password"),
            ExtractionError::CorruptedArchive(s) => write!(f, "Corrupted archive: {}", s),
            ExtractionError::IoError(s) => write!(f, "IO error: {}", s),
            ExtractionError::MultiPartMissing(s) => write!(f, "Missing part: {}", s),
        }
    }
}

/// Archive extractor with progress reporting
pub struct ArchiveExtractor {
    app_handle: AppHandle,
}

impl ArchiveExtractor {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    /// Main extraction function
    pub async fn extract(
        &self,
        archive_paths: Vec<PathBuf>,
        destination: PathBuf,
        password: Option<String>,
        game_id: String,
    ) -> Result<ExtractionResult, ExtractionError> {
        if archive_paths.is_empty() {
            return Err(ExtractionError::IoError("No archive paths provided".to_string()));
        }

        // Ensure destination exists
        std::fs::create_dir_all(&destination)
            .map_err(|e| ExtractionError::IoError(format!("Failed to create destination: {}", e)))?;

        let first_path = &archive_paths[0];
        let format = Self::detect_format(first_path);

        log::info!("[ArchiveExtractor] Extracting {:?} format to {:?}", format, destination);

        match format {
            ArchiveFormat::SevenZip => {
                self.extract_7z(&archive_paths, &destination, password.as_deref(), &game_id).await
            }
            ArchiveFormat::Rar => {
                self.extract_rar(&archive_paths, &destination, password.as_deref(), &game_id).await
            }
            ArchiveFormat::Zip => {
                self.extract_zip(first_path, &destination, password.as_deref(), &game_id).await
            }
            ArchiveFormat::Unknown => {
                Err(ExtractionError::UnsupportedFormat(
                    first_path.extension()
                        .map(|e| e.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ))
            }
        }
    }

    /// Detect archive format from file extension
    pub fn detect_format(path: &Path) -> ArchiveFormat {
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        if filename.ends_with(".7z") || filename.contains(".7z.") {
            ArchiveFormat::SevenZip
        } else if filename.ends_with(".rar") || filename.contains(".part") && filename.contains(".rar") {
            ArchiveFormat::Rar
        } else if filename.ends_with(".r00") || filename.ends_with(".r01") {
            ArchiveFormat::Rar
        } else if filename.ends_with(".zip") || filename.contains(".zip.") || filename.ends_with(".z01") {
            ArchiveFormat::Zip
        } else {
            ArchiveFormat::Unknown
        }
    }

    /// Detect multi-part pattern
    pub fn detect_multi_part_pattern(filename: &str) -> MultiPartPattern {
        let lower = filename.to_lowercase();

        if lower.contains(".part") && lower.ends_with(".rar") {
            MultiPartPattern::RarPart
        } else if lower.ends_with(".r00") || lower.ends_with(".r01") {
            MultiPartPattern::RarVolume
        } else if lower.contains(".7z.") && lower.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            MultiPartPattern::SevenZipPart
        } else if lower.contains(".zip.") || lower.ends_with(".z01") || lower.ends_with(".z02") {
            MultiPartPattern::ZipSplit
        } else {
            MultiPartPattern::None
        }
    }

    /// Check if a filename is a multi-part archive
    pub fn is_multi_part(filename: &str) -> bool {
        !matches!(Self::detect_multi_part_pattern(filename), MultiPartPattern::None)
    }

    /// Find all parts of a multi-part archive
    pub fn find_all_parts(first_part: &Path) -> Vec<PathBuf> {
        let parent = first_part.parent().unwrap_or(Path::new("."));
        let filename = first_part.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let pattern = Self::detect_multi_part_pattern(filename);
        let mut parts = vec![first_part.to_path_buf()];

        match pattern {
            MultiPartPattern::RarPart => {
                // .part1.rar, .part2.rar, etc.
                if let Some(base) = filename.to_lowercase().find(".part") {
                    let base_name = &filename[..base];

                    if let Ok(entries) = std::fs::read_dir(parent) {
                        for entry in entries.filter_map(|e| e.ok()) {
                            let entry_name = entry.file_name().to_string_lossy().to_string();
                            let entry_lower = entry_name.to_lowercase();

                            if entry_lower.starts_with(&base_name.to_lowercase())
                                && entry_lower.contains(".part")
                                && entry_lower.ends_with(".rar")
                                && entry.path() != first_part
                            {
                                parts.push(entry.path());
                            }
                        }
                    }
                }
            }
            MultiPartPattern::RarVolume => {
                // .rar, .r00, .r01, etc.
                let base_name = filename.trim_end_matches(|c: char| c == '.' || c.is_ascii_digit() || c == 'r');

                if let Ok(entries) = std::fs::read_dir(parent) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let entry_name = entry.file_name().to_string_lossy().to_string();
                        let entry_lower = entry_name.to_lowercase();

                        if entry_lower.starts_with(&base_name.to_lowercase())
                            && (entry_lower.ends_with(".rar") ||
                                entry_lower.chars().rev().take(3).all(|c| c.is_ascii_digit() || c == 'r' || c == '.'))
                            && entry.path() != first_part
                        {
                            parts.push(entry.path());
                        }
                    }
                }
            }
            MultiPartPattern::SevenZipPart => {
                // .7z.001, .7z.002, etc.
                if let Some(base_pos) = filename.to_lowercase().find(".7z.") {
                    let base_name = &filename[..base_pos + 3]; // Include .7z

                    if let Ok(entries) = std::fs::read_dir(parent) {
                        for entry in entries.filter_map(|e| e.ok()) {
                            let entry_name = entry.file_name().to_string_lossy().to_string();

                            if entry_name.starts_with(base_name) && entry.path() != first_part {
                                parts.push(entry.path());
                            }
                        }
                    }
                }
            }
            MultiPartPattern::ZipSplit => {
                // .zip.001, .z01, etc.
                let base_name = filename
                    .trim_end_matches(|c: char| c.is_ascii_digit() || c == '.')
                    .trim_end_matches(".zip")
                    .trim_end_matches(".z");

                if let Ok(entries) = std::fs::read_dir(parent) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let entry_name = entry.file_name().to_string_lossy().to_string();
                        let entry_lower = entry_name.to_lowercase();

                        if entry_lower.starts_with(&base_name.to_lowercase()) && entry.path() != first_part {
                            parts.push(entry.path());
                        }
                    }
                }
            }
            MultiPartPattern::None => {}
        }

        // Sort parts
        parts.sort_by(|a, b| {
            let a_name = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let b_name = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
            natord::compare(a_name, b_name)
        });

        parts
    }

    /// Emit extraction progress
    fn emit_progress(&self, progress: &ExtractionProgress) {
        if let Err(e) = self.app_handle.emit("extraction-progress", progress) {
            log::warn!("[ArchiveExtractor] Failed to emit progress: {}", e);
        }
    }

    /// Emit extraction complete
    fn emit_complete(&self, game_id: &str) {
        if let Err(e) = self.app_handle.emit("extraction-complete", serde_json::json!({ "gameId": game_id })) {
            log::warn!("[ArchiveExtractor] Failed to emit complete: {}", e);
        }
    }

    /// Emit extraction error
    fn emit_error(&self, game_id: &str, error: &str) {
        if let Err(e) = self.app_handle.emit("extraction-error", serde_json::json!({
            "gameId": game_id,
            "error": error
        })) {
            log::warn!("[ArchiveExtractor] Failed to emit error: {}", e);
        }
    }

    /// Extract 7z archive
    async fn extract_7z(
        &self,
        paths: &[PathBuf],
        destination: &Path,
        password: Option<&str>,
        game_id: &str,
    ) -> Result<ExtractionResult, ExtractionError> {
        let first_path = &paths[0];
        log::info!("[ArchiveExtractor] Extracting 7z: {:?}", first_path);

        let dest = destination.to_path_buf();
        let path = first_path.clone();
        let gid = game_id.to_string();
        let app = self.app_handle.clone();
        let pwd = password.map(|s| s.to_string());

        // Run extraction in blocking task
        let result = tokio::task::spawn_blocking(move || -> Result<ExtractionResult, ExtractionError> {
            // Initial progress
            let _ = app.emit("extraction-progress", ExtractionProgress {
                game_id: gid.clone(),
                current_file: "Starting extraction...".to_string(),
                files_done: 0,
                files_total: 0,
                bytes_done: 0,
                bytes_total: 0,
                current_archive: 1,
                total_archives: 1,
            });

            if pwd.is_some() {
                log::warn!("[ArchiveExtractor] Password provided for 7z but sevenz_rust simple API does not support passwords — attempting without");
            }

            let extract_result = sevenz_rust::decompress_file(&path, &dest);

            match extract_result {
                Ok(_) => {
                    // Count extracted files
                    let mut files_count = 0u32;
                    let mut total_size = 0u64;

                    for entry in walkdir::WalkDir::new(&dest).into_iter().filter_map(|e| e.ok()) {
                        if entry.file_type().is_file() {
                            files_count += 1;
                            total_size += entry.metadata().map(|m| m.len()).unwrap_or(0);
                        }
                    }

                    Ok(ExtractionResult {
                        success: true,
                        files_extracted: files_count,
                        total_size,
                        destination: dest.to_string_lossy().to_string(),
                        error: None,
                    })
                }
                Err(e) => {
                    let error_str = e.to_string();
                    if error_str.contains("password") || error_str.contains("Password") {
                        Err(ExtractionError::WrongPassword)
                    } else {
                        Err(ExtractionError::CorruptedArchive(error_str))
                    }
                }
            }
        }).await.map_err(|e| ExtractionError::IoError(e.to_string()))?;

        match &result {
            Ok(_) => self.emit_complete(game_id),
            Err(e) => self.emit_error(game_id, &e.to_string()),
        }

        result
    }


    /// Extract RAR archive (Windows only)
    #[cfg(windows)]
    async fn extract_rar(
        &self,
        paths: &[PathBuf],
        destination: &Path,
        password: Option<&str>,
        game_id: &str,
    ) -> Result<ExtractionResult, ExtractionError> {
        let first_path = &paths[0];
        log::info!("[ArchiveExtractor] Extracting RAR: {:?}", first_path);

        self.extract_rar_with_unrar(first_path, destination, password, game_id).await
    }

    /// Convert a path to Windows extended-length path format (\\?\)
    /// This enables proper Unicode support and paths longer than 260 chars
    #[cfg(windows)]
    fn to_extended_path(path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();

        // Already extended or UNC path - return as-is
        if path_str.starts_with(r"\\?\") {
            log::debug!("[ArchiveExtractor] Path already extended: {}", path_str);
            return path.to_path_buf();
        }
        if path_str.starts_with(r"\\") {
            log::debug!("[ArchiveExtractor] UNC path, not modifying: {}", path_str);
            return path.to_path_buf();
        }

        // Canonicalize returns \\?\ prefix on Windows, so just use it directly
        if let Ok(canonical) = path.canonicalize() {
            log::debug!("[ArchiveExtractor] Canonicalized path: {:?}", canonical);
            return canonical;
        }

        // Fallback: manually add prefix for non-existent paths
        let result = PathBuf::from(format!(r"\\?\{}", path.display()));
        log::debug!("[ArchiveExtractor] Manual extended path: {:?}", result);
        result
    }

    /// Extract RAR using the unrar library
    #[cfg(windows)]
    async fn extract_rar_with_unrar(
        &self,
        first_path: &Path,
        destination: &Path,
        password: Option<&str>,
        game_id: &str,
    ) -> Result<ExtractionResult, ExtractionError> {
        use unrar::Archive;

        // NOTE: Do NOT use extended-length paths (\\?\) here.
        // The unrar native library does not handle \\?\ prefix correctly for
        // the extraction destination, causing file creation failures that
        // manifest as false "corrupted archive" errors.
        let dest = destination.to_path_buf();
        let path = first_path.to_path_buf();

        log::info!("[ArchiveExtractor] RAR extraction - Archive: {:?}, Dest: {:?}", path, dest);

        let pwd = password.map(|s| s.to_string());
        let gid = game_id.to_string();
        let app = self.app_handle.clone();

        let result = tokio::task::spawn_blocking(move || -> Result<ExtractionResult, ExtractionError> {
            // Build archive with optional password
            let archive = if let Some(ref p) = pwd {
                Archive::with_password(&path, p)
            } else {
                Archive::new(&path)
            };

            let mut archive = archive
                .open_for_processing()
                .map_err(|e| {
                    let err_str = e.to_string();
                    log::error!("[ArchiveExtractor] RAR open_for_processing error: {}", err_str);
                    let lower = err_str.to_lowercase();
                    if lower.contains("password") || lower.contains("encrypted")
                        || lower.contains("bad data") || lower.contains("wrong")
                    {
                        ExtractionError::PasswordRequired
                    } else {
                        ExtractionError::CorruptedArchive(err_str)
                    }
                })?;

            let mut files_extracted = 0u32;
            let mut total_size = 0u64;

            // Process entries
            while let Some(header) = archive.read_header()
                .map_err(|e| ExtractionError::CorruptedArchive(e.to_string()))?
            {
                let entry = header.entry();
                let unpacked = entry.unpacked_size;
                let is_file = entry.is_file();

                log::info!("[ArchiveExtractor] Entry: {:?} | is_file={} | size={}", entry.filename, is_file, unpacked);

                // Emit progress
                let _ = app.emit("extraction-progress", ExtractionProgress {
                    game_id: gid.clone(),
                    current_file: entry.filename.to_string_lossy().to_string(),
                    files_done: files_extracted,
                    files_total: 0,
                    bytes_done: total_size,
                    bytes_total: 0,
                    current_archive: 1,
                    total_archives: 1,
                });

                // extract_with_base() passes dest as DestPath (directory) via RARProcessFileW.
                // The previously used extract_to() passes the path as DestName (full output
                // filename), so passing a directory caused "Could not create file" every time.
                archive = header.extract_with_base(&dest).map_err(|e| {
                    let err_str = e.to_string();
                    log::error!("[ArchiveExtractor] RAR extract_with_base error: {}", err_str);
                    let lower = err_str.to_lowercase();
                    if lower.contains("password") || lower.contains("bad data")
                        || lower.contains("crc") || lower.contains("wrong")
                        || lower.contains("incorrect")
                    {
                        ExtractionError::WrongPassword
                    } else {
                        ExtractionError::CorruptedArchive(err_str)
                    }
                })?;

                if is_file {
                    files_extracted += 1;
                    total_size += unpacked;
                }
            }

            Ok(ExtractionResult {
                success: true,
                files_extracted,
                total_size,
                destination: dest.to_string_lossy().to_string(),
                error: None,
            })
        }).await.map_err(|e| ExtractionError::IoError(e.to_string()))?;

        match &result {
            Ok(_) => self.emit_complete(game_id),
            Err(e) => self.emit_error(game_id, &e.to_string()),
        }

        result
    }

    /// RAR extraction stub for non-Windows platforms
    #[cfg(not(windows))]
    async fn extract_rar(
        &self,
        _paths: &[PathBuf],
        _destination: &Path,
        _password: Option<&str>,
        game_id: &str,
    ) -> Result<ExtractionResult, ExtractionError> {
        self.emit_error(game_id, "RAR extraction is only supported on Windows");
        Err(ExtractionError::UnsupportedFormat("RAR extraction requires Windows".to_string()))
    }

    /// Extract ZIP archive
    async fn extract_zip(
        &self,
        path: &Path,
        destination: &Path,
        password: Option<&str>,
        game_id: &str,
    ) -> Result<ExtractionResult, ExtractionError> {
        use std::io::Read;

        log::info!("[ArchiveExtractor] Extracting ZIP: {:?}", path);

        let file = std::fs::File::open(path)
            .map_err(|e| ExtractionError::IoError(format!("Failed to open zip: {}", e)))?;

        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| ExtractionError::CorruptedArchive(e.to_string()))?;

        let total_files = archive.len() as u32;
        let mut files_extracted = 0u32;
        let mut total_size = 0u64;

        for i in 0..archive.len() {
            let mut file = if let Some(pwd) = password {
                archive.by_index_decrypt(i, pwd.as_bytes())
                    .map_err(|e| {
                        let err_str = e.to_string();
                        if err_str.contains("password") || err_str.contains("invalid") {
                            ExtractionError::WrongPassword
                        } else {
                            ExtractionError::CorruptedArchive(err_str)
                        }
                    })?
            } else {
                archive.by_index(i)
                    .map_err(|e| ExtractionError::CorruptedArchive(e.to_string()))?
            };

            let outpath = match file.enclosed_name() {
                Some(path) => destination.join(path),
                None => continue,
            };

            // Emit progress
            self.emit_progress(&ExtractionProgress {
                game_id: game_id.to_string(),
                current_file: file.name().to_string(),
                files_done: files_extracted,
                files_total: total_files,
                bytes_done: total_size,
                bytes_total: 0,
                current_archive: 1,
                total_archives: 1,
            });

            if file.is_dir() {
                std::fs::create_dir_all(&outpath)
                    .map_err(|e| ExtractionError::IoError(e.to_string()))?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(p)
                            .map_err(|e| ExtractionError::IoError(e.to_string()))?;
                    }
                }

                let mut outfile = std::fs::File::create(&outpath)
                    .map_err(|e| ExtractionError::IoError(e.to_string()))?;

                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| ExtractionError::IoError(e.to_string()))?;

                total_size += file.size();
            }

            files_extracted += 1;
        }

        self.emit_complete(game_id);

        Ok(ExtractionResult {
            success: true,
            files_extracted,
            total_size,
            destination: destination.to_string_lossy().to_string(),
            error: None,
        })
    }
}

/// Natural sorting comparison for file names
mod natord {
    pub fn compare(a: &str, b: &str) -> std::cmp::Ordering {
        let mut a_iter = a.chars().peekable();
        let mut b_iter = b.chars().peekable();

        loop {
            match (a_iter.peek(), b_iter.peek()) {
                (None, None) => return std::cmp::Ordering::Equal,
                (None, Some(_)) => return std::cmp::Ordering::Less,
                (Some(_), None) => return std::cmp::Ordering::Greater,
                (Some(&ac), Some(&bc)) => {
                    if ac.is_ascii_digit() && bc.is_ascii_digit() {
                        // Compare numbers
                        let mut a_num = String::new();
                        let mut b_num = String::new();

                        while let Some(&c) = a_iter.peek() {
                            if c.is_ascii_digit() {
                                a_num.push(c);
                                a_iter.next();
                            } else {
                                break;
                            }
                        }

                        while let Some(&c) = b_iter.peek() {
                            if c.is_ascii_digit() {
                                b_num.push(c);
                                b_iter.next();
                            } else {
                                break;
                            }
                        }

                        let a_val: u64 = a_num.parse().unwrap_or(0);
                        let b_val: u64 = b_num.parse().unwrap_or(0);

                        match a_val.cmp(&b_val) {
                            std::cmp::Ordering::Equal => continue,
                            other => return other,
                        }
                    } else {
                        // Compare characters
                        let al = ac.to_ascii_lowercase();
                        let bl = bc.to_ascii_lowercase();

                        match al.cmp(&bl) {
                            std::cmp::Ordering::Equal => {
                                a_iter.next();
                                b_iter.next();
                            }
                            other => return other,
                        }
                    }
                }
            }
        }
    }
}

/// Delete archive files after successful extraction
pub async fn delete_archives(paths: &[PathBuf]) -> Result<(), String> {
    for path in paths {
        if path.exists() {
            tokio::fs::remove_file(path)
                .await
                .map_err(|e| format!("Failed to delete {}: {}", path.display(), e))?;
            log::info!("[ArchiveExtractor] Deleted archive: {:?}", path);
        }
    }
    Ok(())
}
