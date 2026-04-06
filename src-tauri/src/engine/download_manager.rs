use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use regex::Regex;
use scraper::{Html, Selector};
use tokio::sync::Mutex;
use futures_util::StreamExt;

use crate::config::hosts::{
    DetectedHost, DownloadError, DownloadErrorType, ErrorCondition,
    HostResolver, HostsConfig, SmartDownloadResult,
};
use crate::config::{ExtractionMethodType, ResolutionStep, Transformation};
use crate::utils::http_client::HttpClient;

use super::host_detector;
use super::download_tracker::DownloadSignal;

/// Download manager that orchestrates smart downloads
pub struct DownloadManager {
    http_client: HttpClient,
}

/// Context maintained during resolution
struct ResolutionContext {
    current_url: String,
    current_content: String,
    variables: HashMap<String, String>,
}

impl DownloadManager {
    pub fn new(http_client: HttpClient) -> Self {
        Self { http_client }
    }

    /// Detect the host from a URL using the provided hosts config
    pub fn detect_host(&self, url: &str, hosts_config: Option<&HostsConfig>) -> DetectedHost {
        host_detector::detect_host(url, hosts_config)
    }

    /// Attempt a smart download using the provided hosts config
    pub async fn smart_download(
        &self,
        url: &str,
        filename_hint: Option<String>,
        hosts_config: Option<&HostsConfig>,
    ) -> SmartDownloadResult {
        log::info!("[DownloadManager] smart_download for URL: {}", url);

        // 1. Detect the host
        let (detected, host_config) = host_detector::detect_host_with_config(url, hosts_config);
        log::info!("[DownloadManager] Detected host: {:?}", detected.host_id);

        // 2. Check if this is a browser-only host
        if let Some(config) = &host_config {
            if config.browser_only {
                let reason = config.browser_only_reason.clone()
                    .unwrap_or_else(|| "This host requires a browser".to_string());
                log::info!("[DownloadManager] Host is browser_only: {}", reason);

                return SmartDownloadResult {
                    success: false,
                    file_path: None,
                    file_size: None,
                    error: Some(DownloadError {
                        error_type: DownloadErrorType::BrowserRequired,
                        message: reason,
                        recoverable: true,
                    }),
                    fallback_url: Some(url.to_string()),
                };
            }
        }

        // 3. If host has a resolver, try to resolve and download
        if let Some(config) = host_config {
            if let Some(resolver) = &config.resolver {
                match self.resolve_download_url(url, resolver).await {
                    Ok(direct_url) => {
                        // Check for error conditions in the resolved content
                        if let Some(error) = self.check_error_conditions(&direct_url, &config.error_conditions).await {
                            return SmartDownloadResult {
                                success: false,
                                file_path: None,
                                file_size: None,
                                error: Some(error),
                                fallback_url: Some(url.to_string()),
                            };
                        }

                        // 3. Download the file
                        match self.download_file(&direct_url, filename_hint.clone(), None).await {
                            Ok((file_path, file_size, _actual_filename)) => {
                                return SmartDownloadResult {
                                    success: true,
                                    file_path: Some(file_path),
                                    file_size: Some(file_size),
                                    error: None,
                                    fallback_url: None,
                                };
                            }
                            Err(e) => {
                                // Download failed - return error with resolved URL as fallback
                                return SmartDownloadResult {
                                    success: false,
                                    file_path: None,
                                    file_size: None,
                                    error: Some(DownloadError {
                                        error_type: DownloadErrorType::NetworkError,
                                        message: e,
                                        recoverable: true,
                                    }),
                                    fallback_url: Some(direct_url),
                                };
                            }
                        }
                    }
                    Err(e) => {
                        return SmartDownloadResult {
                            success: false,
                            file_path: None,
                            file_size: None,
                            error: Some(DownloadError {
                                error_type: DownloadErrorType::ResolutionFailed,
                                message: e,
                                recoverable: true,
                            }),
                            fallback_url: Some(url.to_string()),
                        };
                    }
                }
            }
        }

        // 4. No resolver - try direct download anyway
        match self.download_file(url, filename_hint, None).await {
            Ok((file_path, file_size, _actual_filename)) => SmartDownloadResult {
                success: true,
                file_path: Some(file_path),
                file_size: Some(file_size),
                error: None,
                fallback_url: None,
            },
            Err(e) => SmartDownloadResult {
                success: false,
                file_path: None,
                file_size: None,
                error: Some(DownloadError {
                    error_type: DownloadErrorType::NetworkError,
                    message: e,
                    recoverable: true,
                }),
                fallback_url: Some(url.to_string()),
            },
        }
    }

    /// Resolve a URL through the host-level resolver for size probing.
    /// Returns the direct download URL if the host has a non-browser resolver,
    /// or the original URL if no resolver applies.
    /// This mirrors what `smart_download` does but without actually downloading.
    pub async fn resolve_for_probe(
        &self,
        url: &str,
        hosts_config: Option<&HostsConfig>,
    ) -> String {
        let (_detected, host_config) = host_detector::detect_host_with_config(url, hosts_config);
        if let Some(config) = host_config {
            // Skip browser-only hosts — their URLs can't be resolved statically
            if config.browser_only {
                return url.to_string();
            }
            if let Some(resolver) = &config.resolver {
                if let Ok(direct_url) = self.resolve_download_url(url, resolver).await {
                    return direct_url;
                }
            }
        }
        url.to_string()
    }

    /// Resolve download URL using host resolver steps
    async fn resolve_download_url(
        &self,
        url: &str,
        resolver: &HostResolver,
    ) -> Result<String, String> {
        let mut context = ResolutionContext {
            current_url: url.to_string(),
            current_content: String::new(),
            variables: HashMap::new(),
        };

        // Store original URL as a variable
        context.variables.insert("url".to_string(), url.to_string());
        context.variables.insert("value".to_string(), url.to_string());

        for step in &resolver.steps {
            self.execute_step(step, &mut context, &resolver.headers).await?;
        }

        Ok(context.current_url)
    }

    /// Execute a single resolution step
    async fn execute_step(
        &self,
        step: &ResolutionStep,
        context: &mut ResolutionContext,
        default_headers: &Option<HashMap<String, String>>,
    ) -> Result<(), String> {
        match step {
            ResolutionStep::Fetch { follow_redirects: _, headers, timeout_ms: _ } => {
                // Merge headers
                let merged_headers = merge_headers(default_headers, headers);
                let url = substitute_variables(&context.current_url, &context.variables);

                let content = self.http_client
                    .get_with_headers(&url, &merged_headers)
                    .await?;

                context.current_content = content;
                context.current_url = url;
            }

            ResolutionStep::Extract { method, pattern, group, selector, attribute, fallback } => {
                let extracted = self.extract_value(
                    &context.current_content,
                    &context.current_url,
                    method,
                    pattern,
                    group,
                    selector,
                    attribute,
                    fallback,
                )?;

                context.variables.insert("value".to_string(), extracted.clone());
                context.current_url = extracted;
            }

            ResolutionStep::Transform { transformations } => {
                let value = context.variables.get("value")
                    .cloned()
                    .unwrap_or_else(|| context.current_url.clone());

                let transformed = self.apply_transformations(&value, transformations, &context.variables)?;

                context.variables.insert("value".to_string(), transformed.clone());
                context.current_url = transformed;
            }

            ResolutionStep::Wait { duration_ms } => {
                tokio::time::sleep(tokio::time::Duration::from_millis(*duration_ms)).await;
            }

            ResolutionStep::Custom { _name, _params } => {
                // Custom steps not implemented yet
            }
        }

        Ok(())
    }

    /// Extract a value using the specified method
    fn extract_value(
        &self,
        content: &str,
        url: &str,
        method: &ExtractionMethodType,
        pattern: &Option<String>,
        group: &Option<usize>,
        selector: &Option<String>,
        attribute: &Option<String>,
        fallback: &Option<String>,
    ) -> Result<String, String> {
        let result = match method {
            ExtractionMethodType::Regex => {
                // Can extract from URL or content
                let source = if content.is_empty() { url } else { content };
                self.extract_with_regex(source, pattern, group)?
            }

            ExtractionMethodType::Selector => {
                self.extract_with_selector(content, selector, attribute)?
            }

            ExtractionMethodType::JsonPath => {
                self.extract_with_jsonpath(content, pattern)?
            }

            ExtractionMethodType::Text => {
                content.to_string()
            }

            ExtractionMethodType::XPath => {
                return Err("XPath extraction not supported".to_string());
            }
        };

        if result.is_empty() {
            if let Some(fb) = fallback {
                return Ok(fb.clone());
            }
            return Err("Extraction returned empty result".to_string());
        }

        Ok(result)
    }

    /// Extract using regex
    fn extract_with_regex(
        &self,
        content: &str,
        pattern: &Option<String>,
        group: &Option<usize>,
    ) -> Result<String, String> {
        let pat = pattern.as_ref()
            .ok_or_else(|| "Regex extraction requires 'pattern'".to_string())?;

        let re = Regex::new(pat)
            .map_err(|e| format!("Invalid regex '{}': {}", pat, e))?;

        if let Some(captures) = re.captures(content) {
            let grp = group.unwrap_or(1);
            captures.get(grp)
                .map(|m| m.as_str().to_string())
                .ok_or_else(|| format!("Regex group {} not found", grp))
        } else {
            Err(format!("No match for pattern: {}", pat))
        }
    }

    /// Extract using CSS selector
    fn extract_with_selector(
        &self,
        content: &str,
        selector: &Option<String>,
        attribute: &Option<String>,
    ) -> Result<String, String> {
        let sel = selector.as_ref()
            .ok_or_else(|| "Selector extraction requires 'selector'".to_string())?;

        let document = Html::parse_document(content);
        let css_selector = Selector::parse(sel)
            .map_err(|e| format!("Invalid CSS selector '{}': {:?}", sel, e))?;

        let element = document.select(&css_selector).next()
            .ok_or_else(|| format!("Selector '{}' not found", sel))?;

        if let Some(attr) = attribute {
            element.value().attr(attr)
                .map(|s| s.to_string())
                .ok_or_else(|| format!("Attribute '{}' not found", attr))
        } else {
            Ok(element.text().collect::<String>())
        }
    }

    /// Extract using JSONPath
    fn extract_with_jsonpath(
        &self,
        content: &str,
        path: &Option<String>,
    ) -> Result<String, String> {
        let json_path = path.as_ref()
            .ok_or_else(|| "JSONPath extraction requires 'pattern'".to_string())?;

        let json: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| format!("Invalid JSON: {}", e))?;

        // Use the jsonpath_rust::find method
        let jp = jsonpath_rust::JsonPath::try_from(json_path.as_str())
            .map_err(|e| format!("Invalid JSONPath '{}': {:?}", json_path, e))?;

        let results = jp.find(&json);

        // Results is a Value - check if it's an array and get first element
        match results {
            serde_json::Value::Array(arr) if !arr.is_empty() => {
                match &arr[0] {
                    serde_json::Value::String(s) => Ok(s.clone()),
                    other => Ok(other.to_string().trim_matches('"').to_string()),
                }
            }
            serde_json::Value::String(s) => Ok(s.clone()),
            serde_json::Value::Null => Err(format!("No match for JSONPath: {}", json_path)),
            other => Ok(other.to_string().trim_matches('"').to_string()),
        }
    }

    /// Apply transformations to a value
    fn apply_transformations(
        &self,
        value: &str,
        transformations: &[Transformation],
        variables: &HashMap<String, String>,
    ) -> Result<String, String> {
        let mut current = value.to_string();

        for transform in transformations {
            current = match transform {
                Transformation::Template { template } => {
                    substitute_variables(template, variables)
                }

                Transformation::Replace { pattern, replacement, regex } => {
                    if *regex {
                        let re = Regex::new(pattern)
                            .map_err(|e| format!("Invalid regex: {}", e))?;
                        re.replace_all(&current, replacement.as_str()).to_string()
                    } else {
                        current.replace(pattern, replacement)
                    }
                }

                Transformation::Trim => current.trim().to_string(),
                Transformation::Lowercase => current.to_lowercase(),
                Transformation::Uppercase => current.to_uppercase(),

                Transformation::StripPrefix { prefix } => {
                    current.strip_prefix(prefix).unwrap_or(&current).to_string()
                }

                Transformation::StripSuffix { suffix } => {
                    current.strip_suffix(suffix).unwrap_or(&current).to_string()
                }

                Transformation::Append { text } => format!("{}{}", current, text),
                Transformation::Prepend { text } => format!("{}{}", text, current),

                Transformation::Default { value: default } => {
                    if current.is_empty() { default.clone() } else { current }
                }

                _ => current, // Other transformations not needed for download resolution
            };
        }

        Ok(current)
    }

    /// Check for error conditions in response
    async fn check_error_conditions(
        &self,
        url: &str,
        conditions: &[ErrorCondition],
    ) -> Option<DownloadError> {
        if conditions.is_empty() {
            return None;
        }

        // Fetch the page to check for errors
        let content = match self.http_client.get_with_headers(url, &None).await {
            Ok(c) => c,
            Err(_) => return None, // If we can't fetch, let the download attempt handle it
        };

        for condition in conditions {
            let matched = if let Some(pattern) = &condition.pattern {
                // Regex pattern match
                if let Ok(re) = Regex::new(pattern) {
                    re.is_match(&content)
                } else {
                    false
                }
            } else if let Some(selector) = &condition.selector {
                // CSS selector match
                let doc = Html::parse_document(&content);
                if let Ok(sel) = Selector::parse(selector) {
                    let element = doc.select(&sel).next();
                    if let Some(elem) = element {
                        if let Some(contains) = &condition.contains {
                            elem.text().collect::<String>().contains(contains)
                        } else {
                            true // Element exists
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if matched {
                return Some(DownloadError {
                    error_type: DownloadErrorType::from(condition.error_type.as_str()),
                    message: condition.message.clone(),
                    recoverable: true,
                });
            }
        }

        None
    }

    /// Download a file to the specified folder (legacy non-streaming version)
    /// Returns (file_path, file_size, actual_filename)
    pub async fn download_file(
        &self,
        url: &str,
        filename_hint: Option<String>,
        download_dir: Option<PathBuf>,
    ) -> Result<(String, u64, String), String> {
        // Get download folder - use provided or fallback to user Downloads
        let downloads_dir = download_dir
            .or_else(|| dirs::download_dir())
            .ok_or_else(|| "Could not find Downloads folder".to_string())?;

        // Download the file
        let response = reqwest::get(url)
            .await
            .map_err(|e| format!("Download request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Download failed with status: {}", response.status()));
        }

        // Determine filename - priority: hint > Content-Disposition > URL > default
        let filename = filename_hint
            .or_else(|| extract_filename_from_content_disposition(response.headers()))
            .or_else(|| extract_filename_from_url(url))
            .unwrap_or_else(|| "download".to_string());

        log::info!("[DownloadManager] Using filename: {}", filename);

        let file_path = downloads_dir.join(&filename);

        // Ensure unique filename
        let final_path = ensure_unique_path(file_path);
        let actual_filename = final_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&filename)
            .to_string();

        // Stream to file
        let bytes = response.bytes()
            .await
            .map_err(|e| format!("Failed to read download: {}", e))?;

        let file_size = bytes.len() as u64;

        let mut file = tokio::fs::File::create(&final_path)
            .await
            .map_err(|e| format!("Failed to create file: {}", e))?;

        use tokio::io::AsyncWriteExt;
        file.write_all(&bytes)
            .await
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok((final_path.to_string_lossy().to_string(), file_size, actual_filename))
    }
}

/// Download result for streaming downloads
pub struct StreamingDownloadResult {
    pub file_path: String,
    pub file_size: u64,
    pub actual_filename: String,
}

/// Result when download is paused or cancelled
#[derive(Debug)]
pub struct DownloadPausedInfo {
    pub file_path: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub actual_filename: String,
}

/// Error type that can carry pause/cancel info
#[derive(Debug)]
pub enum StreamingDownloadError {
    Paused(DownloadPausedInfo),
    Cancelled,
    Error(String),
}

/// Progress callback type
pub type ProgressCallback = Box<dyn Fn(u64, u64, u64) + Send + Sync>;

/// Returns true if the IO error is a disk-full / out-of-space error.
fn is_disk_full(e: &std::io::Error) -> bool {
    // Stable kind check (Rust 1.74+)
    if e.kind() == std::io::ErrorKind::StorageFull {
        return true;
    }
    // Raw OS error: Windows ERROR_DISK_FULL=112, ERROR_HANDLE_DISK_FULL=39; Unix ENOSPC=28
    if let Some(code) = e.raw_os_error() {
        #[cfg(windows)]
        if code == 112 || code == 39 { return true; }
        #[cfg(unix)]
        if code == 28 { return true; }
    }
    // String fallback for anything else
    let s = e.to_string().to_lowercase();
    s.contains("no space left") || s.contains("disk is full")
        || s.contains("not enough space") || s.contains("there is not enough space")
        || s.contains("insufficient")
}

/// Wrap an IO error into a `StreamingDownloadError`, tagging disk-full with the
/// `disk_space:` prefix so the frontend can show a specific message.
fn streaming_io_error(e: std::io::Error, context: &str) -> StreamingDownloadError {
    if is_disk_full(&e) {
        StreamingDownloadError::Error(
            "disk_space: Not enough disk space. Free up space on the destination drive and retry.".to_string()
        )
    } else {
        StreamingDownloadError::Error(format!("{}: {}", context, e))
    }
}

/// Streaming download with pause/cancel support
/// This is the proper download function that should be used for tracked downloads
/// Returns Ok on success, or Err with detailed info on pause/cancel/error
pub async fn streaming_download(
    url: &str,
    filename_hint: Option<String>,
    download_dir: PathBuf,
    download_id: String,
    signals: Arc<Mutex<HashMap<String, DownloadSignal>>>,
    progress_callback: Option<ProgressCallback>,
    cookies: Option<String>,
) -> Result<StreamingDownloadResult, StreamingDownloadError> {
    use std::io::Write;
    use reqwest::header::{HeaderMap, HeaderValue, COOKIE};

    log::info!("[StreamingDownload] Starting download: {} -> {:?}", url, download_dir);

    // Create client with proper headers
    let mut headers = HeaderMap::new();
    headers.insert("User-Agent", HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"));
    if let Some(ref c) = cookies {
        if let Ok(v) = HeaderValue::from_str(c) {
            headers.insert(COOKIE, v);
        }
    }

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| StreamingDownloadError::Error(format!("Failed to create client: {}", e)))?;

    let response = client.get(url)
        .send()
        .await
        .map_err(|e| StreamingDownloadError::Error(format!("Request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(StreamingDownloadError::Error(format!("HTTP error: {}", response.status())));
    }

    // Get total size
    let total_bytes = response.content_length().unwrap_or(0);

    // Determine filename
    let filename = filename_hint
        .or_else(|| extract_filename_from_content_disposition(response.headers()))
        .or_else(|| extract_filename_from_url(url))
        .unwrap_or_else(|| "download".to_string());

    log::info!("[StreamingDownload] Filename: {}, Total: {} bytes", filename, total_bytes);

    // Ensure download dir exists
    if !download_dir.exists() {
        std::fs::create_dir_all(&download_dir)
            .map_err(|e| streaming_io_error(e, "Failed to create download directory"))?;
    }

    let file_path = download_dir.join(&filename);
    let final_path = ensure_unique_path(file_path);
    let actual_filename = final_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&filename)
        .to_string();
    let final_path_str = final_path.to_string_lossy().to_string();

    // Create file
    let mut file = std::fs::File::create(&final_path)
        .map_err(|e| streaming_io_error(e, "Failed to create file"))?;

    // Stream download
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_progress_time = std::time::Instant::now();
    let mut last_downloaded = 0u64;

    while let Some(chunk_result) = stream.next().await {
        // Check for pause/cancel signals
        {
            let signals_guard = signals.lock().await;
            if let Some(signal) = signals_guard.get(&download_id) {
                match signal {
                    DownloadSignal::Pause => {
                        log::info!("[StreamingDownload] Paused at {} / {} bytes", downloaded, total_bytes);
                        // Flush and close file handle
                        let _ = file.flush();
                        drop(file);
                        // Return detailed pause info for resume
                        return Err(StreamingDownloadError::Paused(DownloadPausedInfo {
                            file_path: final_path_str,
                            downloaded_bytes: downloaded,
                            total_bytes,
                            actual_filename: actual_filename.clone(),
                        }));
                    }
                    DownloadSignal::Cancel => {
                        log::info!("[StreamingDownload] Cancelled at {} bytes", downloaded);
                        drop(file);
                        // Delete partial file
                        let _ = std::fs::remove_file(&final_path);
                        return Err(StreamingDownloadError::Cancelled);
                    }
                    DownloadSignal::Continue => {}
                }
            }
        }

        let chunk = chunk_result.map_err(|e| StreamingDownloadError::Error(format!("Network error: {}", e)))?;
        file.write_all(&chunk).map_err(|e| streaming_io_error(e, "Write error"))?;
        downloaded += chunk.len() as u64;

        // Emit progress every 250ms
        if last_progress_time.elapsed() >= std::time::Duration::from_millis(250) {
            let speed = ((downloaded - last_downloaded) as f64 / last_progress_time.elapsed().as_secs_f64()) as u64;

            if let Some(ref cb) = progress_callback {
                cb(downloaded, total_bytes, speed);
            }

            last_progress_time = std::time::Instant::now();
            last_downloaded = downloaded;
        }
    }

    file.flush().map_err(|e| streaming_io_error(e, "Flush error"))?;

    log::info!("[StreamingDownload] Completed: {} ({} bytes)", actual_filename, downloaded);

    Ok(StreamingDownloadResult {
        file_path: final_path.to_string_lossy().to_string(),
        file_size: downloaded,
        actual_filename,
    })
}

/// Merge two sets of headers
fn merge_headers(
    base: &Option<HashMap<String, String>>,
    override_headers: &Option<HashMap<String, String>>,
) -> Option<HashMap<String, String>> {
    match (base, override_headers) {
        (None, None) => None,
        (Some(b), None) => Some(b.clone()),
        (None, Some(o)) => Some(o.clone()),
        (Some(b), Some(o)) => {
            let mut merged = b.clone();
            merged.extend(o.clone());
            Some(merged)
        }
    }
}

/// Substitute {variable} placeholders in a string
fn substitute_variables(template: &str, variables: &HashMap<String, String>) -> String {
    let mut result = template.to_string();

    for (key, value) in variables {
        result = result.replace(&format!("{{{}}}", key), value);
    }

    result
}

/// Extract filename from Content-Disposition header
fn extract_filename_from_content_disposition(headers: &reqwest::header::HeaderMap) -> Option<String> {
    let content_disposition = headers.get(reqwest::header::CONTENT_DISPOSITION)?;
    let value = content_disposition.to_str().ok()?;

    log::debug!("[DownloadManager] Content-Disposition: {}", value);

    // Try to extract filename from Content-Disposition header
    // Formats:
    //   attachment; filename="name.ext"
    //   attachment; filename*=UTF-8''name.ext
    //   attachment; filename=name.ext

    // First try filename*= (RFC 5987 encoded)
    if let Some(pos) = value.find("filename*=") {
        let start = pos + "filename*=".len();
        let rest = &value[start..];
        // Skip encoding prefix like UTF-8''
        if let Some(quote_pos) = rest.find("''") {
            let filename = &rest[quote_pos + 2..];
            let filename = filename.split(';').next().unwrap_or(filename).trim();
            if let Ok(decoded) = urlencoding::decode(filename) {
                return Some(decoded.to_string());
            }
        }
    }

    // Try filename= with quotes
    if let Some(pos) = value.find("filename=\"") {
        let start = pos + "filename=\"".len();
        let rest = &value[start..];
        if let Some(end) = rest.find('"') {
            let filename = &rest[..end];
            return Some(filename.to_string());
        }
    }

    // Try filename= without quotes
    if let Some(pos) = value.find("filename=") {
        let start = pos + "filename=".len();
        let rest = &value[start..];
        let filename = rest.split(';').next().unwrap_or(rest).trim();
        if !filename.is_empty() {
            return Some(filename.to_string());
        }
    }

    None
}

/// Extract filename from URL
fn extract_filename_from_url(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let path = parsed.path();

    // Get the last segment
    let filename = path.split('/').last()?;

    if filename.is_empty() || !filename.contains('.') {
        return None;
    }

    // URL decode
    urlencoding::decode(filename).ok().map(|s| s.to_string())
}

/// Ensure a file path is unique by adding (1), (2), etc.
fn ensure_unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let stem = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("download");
    let extension = path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let parent = path.parent().unwrap_or(&path);

    let mut counter = 1;
    loop {
        let new_name = if extension.is_empty() {
            format!("{} ({})", stem, counter)
        } else {
            format!("{} ({}).{}", stem, counter, extension)
        };

        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return new_path;
        }
        counter += 1;
    }
}

