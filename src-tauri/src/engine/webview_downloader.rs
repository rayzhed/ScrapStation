use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Duration;
use std::path::PathBuf;
use std::io::Write;
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri::webview::DownloadEvent;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE};
use futures_util::StreamExt;
use tokio::sync::Mutex as TokioMutex;

use crate::config::hosts::WebViewDownloadConfig;
use crate::engine::download_tracker::{DownloadSignal, DownloadTracker};

/// Result of a webview download attempt
#[derive(Debug, Clone, serde::Serialize)]
pub struct WebViewDownloadResult {
    pub success: bool,
    pub download_url: Option<String>,
    pub file_path: Option<String>,  // Path where file was downloaded (if WebView handled download)
    pub file_size: Option<u64>,     // Size of the downloaded file in bytes
    pub error: Option<String>,
    /// Cookie header string captured from the webview session (probe-only mode).
    /// Passed back to the real download to skip re-authentication.
    pub cookies: Option<String>,
}

/// Event payload for download URL capture
#[derive(Debug, Clone, serde::Deserialize)]
struct DownloadUrlPayload {
    url: String,
}

/// Progress event sent to frontend
#[derive(Debug, Clone, serde::Serialize)]
pub struct DownloadProgressEvent {
    pub id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed: u64,  // bytes per second
}

/// Download state shared between threads
#[derive(Debug, Clone)]
struct DownloadState {
    pub destination_path: Option<PathBuf>,
    pub total_bytes: Option<u64>,
    pub started: bool,
    pub finished: bool,
    pub success: bool,
    pub captured_download_url: Option<String>,  // URL captured from WebView download request
    pub use_reqwest: bool,  // Flag to use reqwest instead of WebView for download
}

/// Handles downloads that require a WebView (JavaScript execution)
pub struct WebViewDownloader {
    app_handle: AppHandle,
}

impl WebViewDownloader {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    /// Extract a filename from a Content-Disposition header value.
    /// Handles both `filename="foo.rar"` and `filename*=UTF-8''foo.rar` forms.
    fn filename_from_content_disposition(header: &str) -> Option<String> {
        let lower = header.to_lowercase();
        // RFC 5987 form takes priority: filename*=UTF-8''name.rar
        if let Some(pos) = lower.find("filename*=") {
            let rest = &header[pos + "filename*=".len()..];
            let value = if let Some(p) = rest.find("''") { &rest[p + 2..] } else { rest };
            let value = value.split(';').next().unwrap_or("").trim().trim_matches('"');
            if !value.is_empty() {
                return Some(
                    urlencoding::decode(value)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| value.to_string()),
                );
            }
        }
        // Plain filename= form
        if let Some(pos) = lower.find("filename=") {
            let rest = &header[pos + "filename=".len()..];
            let value = rest.split(';').next().unwrap_or("").trim().trim_matches('"');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
        None
    }

    /// Guess a file extension from a Content-Type header value.
    fn ext_from_content_type(content_type: &str) -> Option<&'static str> {
        let ct = content_type.split(';').next().unwrap_or("").trim().to_lowercase();
        match ct.as_str() {
            "application/x-rar-compressed"
            | "application/vnd.rar"
            | "application/rar" => Some("rar"),
            "application/x-7z-compressed" => Some("7z"),
            "application/zip" | "application/x-zip-compressed" => Some("zip"),
            "application/gzip" | "application/x-gzip" => Some("gz"),
            "application/x-tar" => Some("tar"),
            _ => None,
        }
    }

    /// Try to get file size via HEAD request
    async fn get_content_length(url: &str) -> Option<u64> {
        let client = reqwest::Client::new();
        match client.head(url).send().await {
            Ok(resp) => resp.content_length(),
            Err(_) => None,
        }
    }

    /// Download a file using reqwest with cookies, emitting progress events
    /// Now checks for pause/cancel signals from the download tracker
    async fn download_with_reqwest(
        &self,
        url: &str,
        destination: &PathBuf,
        cookies: Vec<tauri::webview::Cookie<'static>>,
        download_id: Option<String>,
    ) -> Result<(PathBuf, u64), String> {
        log::info!("[WebViewDownloader] Starting reqwest download: {} -> {:?}", url, destination);
        log::info!("[WebViewDownloader] Using {} cookies", cookies.len());

        // Get tracker and signals
        let tracker_state = self.app_handle.try_state::<Arc<TokioMutex<DownloadTracker>>>();
        let signals: Option<Arc<TokioMutex<HashMap<String, DownloadSignal>>>> = if download_id.is_some() {
            tracker_state.as_ref().map(|tracker| {
                let tracker_arc = tracker.inner().clone();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let t = tracker_arc.lock().await;
                        t.get_signals()
                    })
                })
            })
        } else {
            None
        };

        // Build cookie header from extracted cookies
        let cookie_header: String = cookies
            .iter()
            .map(|c| format!("{}={}", c.name(), c.value()))
            .collect::<Vec<_>>()
            .join("; ");

        // Save the resolved URL, file path, and cookies BEFORE starting download
        // This is critical for resume to work correctly
        if let (Some(ref dl_id), Some(ref tracker)) = (&download_id, &tracker_state) {
            let tracker_arc = tracker.inner().clone();
            let t = tracker_arc.lock().await;
            let _ = t.update_resolved_url(dl_id, url).await;
            let _ = t.update_file_path(dl_id, &destination.to_string_lossy()).await;
            if !cookie_header.is_empty() {
                let _ = t.update_cookies(dl_id, &cookie_header).await;
            }
            drop(t);
        }

        log::debug!("[WebViewDownloader] Cookie header: {}", cookie_header);

        // Create reqwest client with cookies
        let mut headers = HeaderMap::new();
        if !cookie_header.is_empty() {
            headers.insert(COOKIE, HeaderValue::from_str(&cookie_header).map_err(|e| e.to_string())?);
        }
        headers.insert("User-Agent", HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| format!("Failed to create client: {}", e))?;

        // Start the download
        let response = client.get(url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let total_size = response.content_length().unwrap_or(0);
        log::info!("[WebViewDownloader] Content-Length: {} bytes", total_size);

        // Resolve the best filename from response headers.
        // CDN URLs (e.g. trashbytes.net/dl/<token>) carry no extension in the path,
        // so we check Content-Disposition first, then fall back to Content-Type.
        let actual_destination: PathBuf = {
            let headers = response.headers();

            let cd_name = headers
                .get("content-disposition")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| Self::filename_from_content_disposition(s));

            let parent = destination.parent().unwrap_or(destination.as_path());

            if let Some(name) = cd_name {
                log::info!("[WebViewDownloader] Filename from Content-Disposition: {}", name);
                parent.join(name)
            } else {
                let current = destination.file_name().and_then(|n| n.to_str()).unwrap_or("download");
                let has_ext = std::path::Path::new(current).extension().is_some();
                if !has_ext {
                    let ext = headers
                        .get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| Self::ext_from_content_type(s));
                    if let Some(ext) = ext {
                        let fixed = format!("{}.{}", current, ext);
                        log::info!("[WebViewDownloader] Added extension from Content-Type: {}", fixed);
                        parent.join(fixed)
                    } else {
                        destination.clone()
                    }
                } else {
                    destination.clone()
                }
            }
        };

        // If the filename changed, update the tracker so the UI and resume logic use the right path
        if actual_destination != *destination {
            if let (Some(ref dl_id), Some(ref tracker)) = (&download_id, &tracker_state) {
                let actual_name = actual_destination
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("download")
                    .to_string();
                let tracker_arc = tracker.inner().clone();
                let t = tracker_arc.lock().await;
                let _ = t.update_filename(dl_id, &actual_name).await;
                let _ = t.update_file_path(dl_id, &actual_destination.to_string_lossy()).await;
            }
        }

        // Ensure parent directory exists
        if let Some(parent) = actual_destination.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            }
        }

        // Create the destination file
        let mut file = std::fs::File::create(&actual_destination)
            .map_err(|e| format!("Failed to create file: {}", e))?;

        // Download with progress tracking
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();
        let mut last_emit_time = std::time::Instant::now();
        let mut last_emit_bytes: u64 = 0;
        let emit_interval = Duration::from_millis(250); // Emit progress every 250ms

        while let Some(chunk_result) = stream.next().await {
            // Check for pause/cancel signals
            if let (Some(ref dl_id), Some(ref sigs)) = (&download_id, &signals) {
                let signal = {
                    let signals_guard = sigs.lock().await;
                    signals_guard.get(dl_id).cloned().unwrap_or(DownloadSignal::Continue)
                };

                match signal {
                    DownloadSignal::Pause => {
                        log::info!("[WebViewDownloader] Download paused at {} / {} bytes: {}", downloaded, total_size, dl_id);
                        // Flush and close file to ensure data is written
                        let _ = file.flush();
                        drop(file);

                        // Save progress to tracker so resume works correctly
                        if let Some(ref tracker) = tracker_state {
                            let tracker_arc = tracker.inner().clone();
                            let t = tracker_arc.lock().await;
                            let _ = t.pause_download_with_progress(dl_id, downloaded, total_size).await;
                        }

                        return Err("PAUSED".to_string());
                    }
                    DownloadSignal::Cancel => {
                        log::info!("[WebViewDownloader] Download cancelled: {}", dl_id);
                        drop(file); // Close file handle
                        // Delete partial file
                        let _ = std::fs::remove_file(&actual_destination);
                        return Err("CANCELLED".to_string());
                    }
                    DownloadSignal::Continue => {}
                }
            }

            let chunk: bytes::Bytes = chunk_result.map_err(|e| format!("Download error: {}", e))?;

            file.write_all(&chunk)
                .map_err(|e| format!("Write error: {}", e))?;

            downloaded += chunk.len() as u64;

            // Emit progress at regular intervals
            if let Some(ref dl_id) = download_id {
                let now = std::time::Instant::now();
                if now.duration_since(last_emit_time) >= emit_interval || downloaded == total_size {
                    let elapsed = now.duration_since(last_emit_time).as_secs_f64();
                    let bytes_since_last = downloaded - last_emit_bytes;
                    let speed = if elapsed > 0.0 {
                        (bytes_since_last as f64 / elapsed) as u64
                    } else {
                        0
                    };

                    let progress = DownloadProgressEvent {
                        id: dl_id.clone(),
                        downloaded_bytes: downloaded,
                        total_bytes: total_size,
                        speed,
                    };

                    if let Err(e) = self.app_handle.emit("download-progress", &progress) {
                        log::warn!("[WebViewDownloader] Failed to emit progress: {}", e);
                    }

                    last_emit_time = now;
                    last_emit_bytes = downloaded;
                }
            }
        }

        // Ensure file is flushed
        file.flush().map_err(|e| format!("Flush error: {}", e))?;

        log::info!("[WebViewDownloader] Download complete: {} bytes", downloaded);

        Ok((actual_destination, downloaded))
    }

    /// Attempt to get download URL using an invisible WebView
    /// download_id is used for progress tracking events.
    /// probe_only: if true, capture CDN URL + return Content-Length without downloading anything.
    pub async fn get_download_url(
        &self,
        url: &str,
        config: &WebViewDownloadConfig,
        download_id: Option<String>,
        probe_only: bool,
    ) -> WebViewDownloadResult {
        log::info!("[WebViewDownloader] Starting for URL: {}", url);

        // Create a unique window label
        let window_id = fastrand::u64(..);
        let window_label = format!("download_webview_{}", window_id);
        let event_name = format!("download-url-captured-{}", window_id);

        // Shared state for captured URL and download result
        let captured_url: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let captured_url_clone = captured_url.clone();
        let captured_url_for_download = captured_url.clone();

        // Shared state for download tracking (path, total size, status)
        let download_state: Arc<Mutex<DownloadState>> = Arc::new(Mutex::new(DownloadState {
            destination_path: None,
            total_bytes: None,
            started: false,
            finished: false,
            success: false,
            captured_download_url: None,
            use_reqwest: true,  // Default to using reqwest for trackable downloads
        }));
        let download_state_for_handler = download_state.clone();
        let download_state_for_finish = download_state.clone();

        // Clone app_handle for use in closure
        let app_handle_for_path = self.app_handle.clone();

        // Listen for the download URL event (from JS interception)
        let event_name_clone = event_name.clone();
        let _unlisten = self.app_handle.listen(&event_name_clone, move |event| {
            if let Ok(payload) = serde_json::from_str::<DownloadUrlPayload>(event.payload()) {
                log::info!("[WebViewDownloader] Received download URL from JS: {}", payload.url);
                if let Ok(mut guard) = captured_url_clone.lock() {
                    *guard = Some(payload.url);
                }
            }
        });

        // Create webview window with on_download handler to intercept downloads
        let parsed_url = match url.parse() {
            Ok(u) => u,
            Err(e) => {
                log::error!("[WebViewDownloader] Invalid URL '{}': {}", url, e);
                return WebViewDownloadResult {
                    success: false,
                    download_url: None,
                    file_path: None,
                    file_size: None,
                    error: Some(format!("Invalid URL '{}': {}", url, e)),
                    cookies: None,
                };
            }
        };
        let webview_result = WebviewWindowBuilder::new(
            &self.app_handle,
            &window_label,
            WebviewUrl::External(parsed_url),
        )
        .title("Download")
        .visible(false)  // Hidden for production
        .inner_size(1024.0, 768.0)
        .on_download(move |_webview, event| {
            match event {
                DownloadEvent::Requested { url, destination: _ } => {
                    // Capture the download URL but CANCEL the WebView download
                    // We will use reqwest with cookies for trackable progress
                    let url_str = url.to_string();
                    log::info!("[WebViewDownloader] Download URL captured: {}", url_str);

                    // Extract filename from URL path for later use
                    let filename = url.path_segments()
                        .and_then(|segments| segments.last())
                        .unwrap_or("download");
                    let filename = urlencoding::decode(filename)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| filename.to_string());

                    // Compute destination path - use app's download folder
                    let download_path = Some(crate::engine::get_download_folder(&app_handle_for_path).join(&filename));

                    // Update download state with captured URL and path
                    if let Ok(mut state) = download_state_for_handler.lock() {
                        state.captured_download_url = Some(url_str.clone());
                        state.destination_path = download_path;
                        state.started = true;
                        state.use_reqwest = true;
                    }

                    // Store URL for tracking (backwards compatibility)
                    if let Ok(mut guard) = captured_url_for_download.lock() {
                        *guard = Some(url_str);
                    }

                    // Return FALSE to cancel WebView download - we'll use reqwest instead
                    false
                }
                DownloadEvent::Finished { url, path, success } => {
                    log::info!("[WebViewDownloader] WebView download event: {} -> {:?} (success: {})", url, path, success);
                    // This shouldn't trigger if we returned false above, but handle it anyway
                    if let Ok(mut state) = download_state_for_finish.lock() {
                        if !state.use_reqwest {
                            state.finished = true;
                            state.success = success;
                            if success {
                                if let Some(p) = path {
                                    state.destination_path = Some(p);
                                }
                            }
                        }
                    }
                    true
                }
                _ => true,
            }
        })
        .build();

        let webview = match webview_result {
            Ok(w) => w,
            Err(e) => {
                log::error!("[WebViewDownloader] Failed to create webview: {}", e);
                return WebViewDownloadResult {
                    success: false,
                    download_url: None,
                    file_path: None,
                    file_size: None,
                    error: Some(format!("Failed to create webview: {}", e)),
                    cookies: None,
                };
            }
        };

        log::info!("[WebViewDownloader] WebView created, waiting for page load...");

        // Initial wait for page to load
        tokio::time::sleep(Duration::from_millis(3000)).await;

        // Inject the capture script with the event name
        let capture_script = self.build_capture_script(&event_name, config);
        if let Err(e) = webview.eval(&capture_script) {
            log::error!("[WebViewDownloader] Failed to inject capture script: {}", e);
        }

        // If we have a wait_for selector, wait for it
        if let Some(wait_selector) = &config.wait_for {
            log::info!("[WebViewDownloader] Waiting for selector: {}", wait_selector);
            self.wait_for_selector(&webview, wait_selector, config.wait_timeout_ms).await;
        }

        // Execute pre-script if provided (e.g., close popups)
        if let Some(pre_script) = &config.pre_script {
            log::info!("[WebViewDownloader] Executing pre-script");
            if let Err(e) = webview.eval(pre_script) {
                log::warn!("[WebViewDownloader] Pre-script failed: {}", e);
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // Execute extraction based on config
        if let Some(extract_script) = &config.extract_url_script {
            log::info!("[WebViewDownloader] Using custom extraction script");
            let full_script = format!(
                r#"
                (async function() {{
                    try {{
                        const url = await (async function() {{ {} }})();
                        if (url) {{
                            window.__TAURI__.event.emit('{}', {{ url: url }});
                        }}
                    }} catch(e) {{
                        console.error('Extraction failed:', e);
                    }}
                }})();
                "#,
                extract_script, event_name
            );
            if let Err(e) = webview.eval(&full_script) {
                log::error!("[WebViewDownloader] Extraction script failed: {}", e);
            }
        } else if let Some(click_selector) = &config.click {
            log::info!("[WebViewDownloader] Clicking selector: {}", click_selector);
            let click_script = format!(
                r#"
                (function() {{
                    const el = document.querySelector('{}');
                    if (el) {{
                        // If it has an href, capture it before clicking
                        if (el.href && !el.href.startsWith('javascript:')) {{
                            window.__TAURI__.event.emit('{}', {{ url: el.href }});
                        }}
                        el.click();
                    }} else {{
                        console.error('Element not found:', '{}');
                    }}
                }})();
                "#,
                click_selector.replace('\'', "\\'"),
                event_name,
                click_selector.replace('\'', "\\'")
            );
            if let Err(e) = webview.eval(&click_script) {
                log::error!("[WebViewDownloader] Click script failed: {}", e);
            }
        }

        // Wait for the download URL to be captured
        // Use the configured timeout (hosts with timers need longer waits)
        let poll_interval = Duration::from_millis(1000);
        let start = std::time::Instant::now();
        let url_capture_timeout = Duration::from_millis(config.wait_timeout_ms.max(60000));
        let click_selector = config.click.clone();

        log::info!("[WebViewDownloader] Waiting for download URL (timeout: {}ms)...", url_capture_timeout.as_millis());

        // Wait for download URL to be captured, periodically re-clicking for timer-based hosts
        let mut last_click_time = std::time::Instant::now();
        let click_retry_interval = Duration::from_secs(5); // Try clicking every 5 seconds

        loop {
            if start.elapsed() > url_capture_timeout {
                log::error!("[WebViewDownloader] Timeout waiting for download URL");
                break;
            }

            let has_url = download_state.lock().ok()
                .map(|s| s.captured_download_url.is_some())
                .unwrap_or(false);

            if has_url {
                log::info!("[WebViewDownloader] Download URL captured, proceeding with cookie extraction...");
                break;
            }

            // For timer-based hosts: periodically try clicking download buttons
            // This handles cases where a timer completes and a new button appears
            if let Some(ref selector) = click_selector {
                if last_click_time.elapsed() >= click_retry_interval {
                    log::debug!("[WebViewDownloader] Re-trying click on selector: {}", selector);
                    let retry_click_script = format!(
                        r#"
                        (function() {{
                            // Try multiple download button selectors
                            const selectors = ['{}', 'a[href*="download"]', 'button[class*="download"]', '#downloadButton', '.download-btn', 'a.btn-download'];
                            for (const sel of selectors) {{
                                const el = document.querySelector(sel);
                                if (el && el.offsetParent !== null) {{ // Check if visible
                                    console.log('[CrackStation] Clicking:', sel);
                                    if (el.href && !el.href.startsWith('javascript:')) {{
                                        window.__TAURI__.event.emit('{}', {{ url: el.href }});
                                    }}
                                    el.click();
                                    break;
                                }}
                            }}
                        }})();
                        "#,
                        selector.replace('\'', "\\'"),
                        event_name
                    );
                    let _ = webview.eval(&retry_click_script);
                    last_click_time = std::time::Instant::now();
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        // Get the captured download URL and destination
        let (captured_download_url, destination_path) = {
            let state = download_state.lock().ok();
            state.map(|s| (s.captured_download_url.clone(), s.destination_path.clone()))
                .unwrap_or((None, None))
        };

        // If we captured a URL, download using reqwest with cookies
        if let Some(download_url) = captured_download_url {

            // ---- PROBE-ONLY MODE: just return Content-Length, no actual download ----
            if probe_only {
                log::info!("[WebViewDownloader] Probe-only: captured CDN URL {}", download_url);

                // Extract cookies from the webview session
                let cookie_header = match download_url.parse::<url::Url>() {
                    Ok(parsed_url) => {
                        webview.cookies_for_url(parsed_url).ok()
                            .map(|cookies| {
                                cookies.iter()
                                    .map(|c| format!("{}={}", c.name(), c.value()))
                                    .collect::<Vec<_>>()
                                    .join("; ")
                            })
                            .unwrap_or_default()
                    }
                    Err(_) => String::new(),
                };

                let _ = webview.close();

                // Bare GET — no Accept-Encoding so server must include Content-Length
                let mut headers = HeaderMap::new();
                if !cookie_header.is_empty() {
                    if let Ok(v) = HeaderValue::from_str(&cookie_header) {
                        headers.insert(COOKIE, v);
                    }
                }
                headers.insert("User-Agent", HeaderValue::from_static(
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
                ));

                let file_size = if let Ok(client) = reqwest::Client::builder()
                    .default_headers(headers)
                    .redirect(reqwest::redirect::Policy::limited(10))
                    .build()
                {
                    client.get(&download_url).send().await
                        .ok()
                        .and_then(|r| if r.status().is_success() { r.content_length() } else { None })
                } else {
                    None
                };

                log::info!("[WebViewDownloader] Probe content-length: {:?}", file_size);
                return WebViewDownloadResult {
                    success: true,
                    download_url: Some(download_url),
                    file_path: None,
                    file_size,
                    error: None,
                    cookies: if cookie_header.is_empty() { None } else { Some(cookie_header) },
                };
            }
            // ---- END PROBE-ONLY MODE ----

            if let Some(destination) = destination_path {
                log::info!("[WebViewDownloader] Extracting cookies for URL: {}", download_url);

                // Extract the actual filename from the download URL and update tracker BEFORE download starts
                if let Some(ref dl_id) = download_id {
                    let actual_filename = destination.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("download")
                        .to_string();

                    log::info!("[WebViewDownloader] Actual filename from URL: {}", actual_filename);

                    // Update tracker SYNCHRONOUSLY to ensure UI updates before download starts
                    if let Some(tracker) = self.app_handle.try_state::<Arc<TokioMutex<DownloadTracker>>>() {
                        let tracker_arc = tracker.inner().clone();
                        let dl_id_clone = dl_id.clone();
                        let filename_clone = actual_filename.clone();

                        // Do this synchronously so UI updates immediately
                        let t = tracker_arc.lock().await;
                        // Update filename to the actual filename from the download URL
                        let _ = t.update_filename(&dl_id_clone, &filename_clone).await;
                        // Mark as resumable since reqwest supports Range header for pause/resume
                        let _ = t.mark_resumable(&dl_id_clone).await;
                        // Ensure status is 'downloading'
                        let _ = t.update_status(&dl_id_clone, crate::engine::DownloadStatus::Downloading).await;
                        drop(t); // Release lock before download starts
                    }
                }

                // Extract cookies from the WebView
                let cookies = match download_url.parse::<url::Url>() {
                    Ok(parsed_url) => {
                        // Get cookies for the download URL domain
                        match webview.cookies_for_url(parsed_url.clone()) {
                            Ok(c) => {
                                log::info!("[WebViewDownloader] Extracted {} cookies", c.len());
                                for cookie in &c {
                                    log::debug!("[WebViewDownloader] Cookie: {}={}", cookie.name(), cookie.value());
                                }
                                c
                            }
                            Err(e) => {
                                log::warn!("[WebViewDownloader] Failed to extract cookies: {}", e);
                                Vec::new()
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("[WebViewDownloader] Failed to parse download URL: {}", e);
                        Vec::new()
                    }
                };

                // Download with reqwest using extracted cookies
                match self.download_with_reqwest(&download_url, &destination, cookies, download_id.clone()).await {
                    Ok((path, size)) => {
                        // Close the webview
                        if let Err(e) = webview.close() {
                            log::warn!("[WebViewDownloader] Failed to close webview: {}", e);
                        }

                        // Update tracker with actual filename and emit events
                        if let Some(ref dl_id) = download_id {
                            let actual_filename = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("download")
                                .to_string();

                            // Update filename in tracker
                            if let Some(tracker) = self.app_handle.try_state::<Arc<TokioMutex<DownloadTracker>>>() {
                                let tracker_arc = tracker.inner().clone();
                                let dl_id_clone = dl_id.clone();
                                let filename_clone = actual_filename.clone();
                                let file_path = path.to_string_lossy().to_string();
                                tokio::spawn(async move {
                                    let t = tracker_arc.lock().await;
                                    let _ = t.update_filename(&dl_id_clone, &filename_clone).await;
                                    let _ = t.complete_download(&dl_id_clone, &file_path, size).await;
                                });
                            }

                            // Emit completion event
                            let _ = self.app_handle.emit("download-complete", serde_json::json!({
                                "id": dl_id,
                                "file_path": path.to_string_lossy()
                            }));
                        }

                        return WebViewDownloadResult {
                            success: true,
                            download_url: Some(download_url),
                            file_path: Some(path.to_string_lossy().to_string()),
                            file_size: Some(size),
                            error: None,
                            cookies: None,
                        };
                    }
                    Err(e) => {
                        // Close the webview first
                        if let Err(close_err) = webview.close() {
                            log::warn!("[WebViewDownloader] Failed to close webview: {}", close_err);
                        }

                        // Handle pause/cancel signals
                        if e == "PAUSED" {
                            log::info!("[WebViewDownloader] Download paused by user");
                            if let Some(ref dl_id) = download_id {
                                if let Some(tracker) = self.app_handle.try_state::<Arc<TokioMutex<DownloadTracker>>>() {
                                    let tracker_arc = tracker.inner().clone();
                                    let dl_id_clone = dl_id.clone();
                                    tokio::spawn(async move {
                                        let t = tracker_arc.lock().await;
                                        let _ = t.update_status(&dl_id_clone, crate::engine::DownloadStatus::Paused).await;
                                    });
                                }
                            }
                            return WebViewDownloadResult {
                                success: false,
                                download_url: Some(download_url),
                                file_path: None,
                                file_size: None,
                                error: Some("Download paused".to_string()),
                                cookies: None,
                            };
                        }

                        if e == "CANCELLED" {
                            log::info!("[WebViewDownloader] Download cancelled by user");
                            // Tracker status already updated by cancel_download command
                            return WebViewDownloadResult {
                                success: false,
                                download_url: Some(download_url),
                                file_path: None,
                                file_size: None,
                                error: Some("Download cancelled".to_string()),
                                cookies: None,
                            };
                        }

                        log::warn!("[WebViewDownloader] Reqwest download failed: {}, falling back to WebView download", e);

                        // Fallback: Use WebView to download directly (no progress tracking but works for protected hosts)
                        return self.download_with_webview_fallback(&download_url, &destination, download_id.clone()).await;
                    }
                }
            }
        }

        // If we reach here, URL capture failed
        log::error!("[WebViewDownloader] No download URL was captured");

        // Close the webview
        if let Err(e) = webview.close() {
            log::warn!("[WebViewDownloader] Failed to close webview: {}", e);
        }

        // Emit error event
        if let Some(ref dl_id) = download_id {
            let _ = self.app_handle.emit("download-error", serde_json::json!({
                "id": dl_id,
                "error": "Failed to capture download URL"
            }));
        }

        WebViewDownloadResult {
            success: false,
            download_url: captured_url.lock().ok().and_then(|g| g.clone()),
            file_path: None,
            file_size: None,
            error: Some("Timeout waiting for download URL to be captured".to_string()),
            cookies: None,
        }
    }

    /// Fallback: Download using WebView when reqwest fails (e.g., due to additional server-side protection)
    /// This won't have real-time progress tracking but will work for hosts that block direct downloads
    async fn download_with_webview_fallback(
        &self,
        download_url: &str,
        destination: &PathBuf,
        download_id: Option<String>,
    ) -> WebViewDownloadResult {
        log::info!("[WebViewDownloader] Fallback: Starting WebView download for: {}", download_url);

        // Create a new WebView that will perform the actual download
        let window_id = fastrand::u64(..);
        let window_label = format!("download_fallback_{}", window_id);

        // Shared state for tracking the fallback download
        let fallback_state: Arc<Mutex<(bool, bool, Option<PathBuf>)>> = Arc::new(Mutex::new((false, false, None)));
        let fallback_state_for_handler = fallback_state.clone();
        let fallback_state_for_finish = fallback_state.clone();

        let destination_clone = destination.clone();

        let parsed_download_url = match download_url.parse() {
            Ok(u) => u,
            Err(e) => {
                log::error!("[WebViewDownloader] Invalid download URL '{}': {}", download_url, e);
                return WebViewDownloadResult {
                    success: false,
                    download_url: Some(download_url.to_string()),
                    file_path: None,
                    file_size: None,
                    error: Some(format!("Invalid download URL '{}': {}", download_url, e)),
                    cookies: None,
                };
            }
        };
        let webview_result = WebviewWindowBuilder::new(
            &self.app_handle,
            &window_label,
            WebviewUrl::External(parsed_download_url),
        )
        .title("Download (Fallback)")
        .visible(false)
        .inner_size(800.0, 600.0)
        .on_download(move |_webview, event| {
            match event {
                DownloadEvent::Requested { url: _, destination: dest } => {
                    log::info!("[WebViewDownloader] Fallback download started -> {:?}", destination_clone);
                    *dest = destination_clone.clone();

                    if let Ok(mut state) = fallback_state_for_handler.lock() {
                        state.0 = true; // started
                    }

                    // Return TRUE to let WebView handle the download
                    true
                }
                DownloadEvent::Finished { url: _, path, success } => {
                    log::info!("[WebViewDownloader] Fallback download finished (success: {})", success);

                    if let Ok(mut state) = fallback_state_for_finish.lock() {
                        state.1 = success; // success
                        state.2 = path;    // path
                    }

                    true
                }
                _ => true,
            }
        })
        .build();

        let webview = match webview_result {
            Ok(w) => w,
            Err(e) => {
                log::error!("[WebViewDownloader] Failed to create fallback webview: {}", e);

                if let Some(ref dl_id) = download_id {
                    let _ = self.app_handle.emit("download-error", serde_json::json!({
                        "id": dl_id,
                        "error": format!("Failed to create fallback webview: {}", e)
                    }));
                }

                return WebViewDownloadResult {
                    success: false,
                    download_url: Some(download_url.to_string()),
                    file_path: None,
                    file_size: None,
                    error: Some(format!("Failed to create fallback webview: {}", e)),
                    cookies: None,
                };
            }
        };

        // Wait for the download to complete (with timeout)
        let download_timeout = Duration::from_secs(600); // 10 minutes
        let poll_interval = Duration::from_millis(1000);
        let start = std::time::Instant::now();

        // Emit indeterminate progress (no real tracking for WebView downloads)
        if let Some(ref dl_id) = download_id {
            let progress = DownloadProgressEvent {
                id: dl_id.clone(),
                downloaded_bytes: 0,
                total_bytes: 0,  // Indeterminate
                speed: 0,
            };
            let _ = self.app_handle.emit("download-progress", &progress);
        }

        loop {
            if start.elapsed() > download_timeout {
                log::error!("[WebViewDownloader] Fallback download timeout");
                break;
            }

            let (started, success, path) = {
                fallback_state.lock().ok()
                    .map(|s| (s.0, s.1, s.2.clone()))
                    .unwrap_or((false, false, None))
            };

            // Check if download finished
            if started && path.is_some() {
                if success {
                    let file_path = match path {
                        Some(p) => p,
                        None => continue,
                    };
                    let file_size = std::fs::metadata(&file_path).map(|m| m.len()).ok();

                    log::info!("[WebViewDownloader] Fallback download completed: {:?} ({:?} bytes)", file_path, file_size);

                    // Close the webview
                    if let Err(e) = webview.close() {
                        log::warn!("[WebViewDownloader] Failed to close fallback webview: {}", e);
                    }

                    // Emit completion
                    if let Some(ref dl_id) = download_id {
                        // Final progress with actual size
                        if let Some(size) = file_size {
                            let progress = DownloadProgressEvent {
                                id: dl_id.clone(),
                                downloaded_bytes: size,
                                total_bytes: size,
                                speed: 0,
                            };
                            let _ = self.app_handle.emit("download-progress", &progress);
                        }

                        let _ = self.app_handle.emit("download-complete", serde_json::json!({
                            "id": dl_id,
                            "file_path": file_path.to_string_lossy()
                        }));
                    }

                    return WebViewDownloadResult {
                        success: true,
                        download_url: Some(download_url.to_string()),
                        file_path: Some(file_path.to_string_lossy().to_string()),
                        file_size,
                        error: None,
                        cookies: None,
                    };
                } else {
                    // Download failed
                    log::error!("[WebViewDownloader] Fallback download failed");

                    if let Err(e) = webview.close() {
                        log::warn!("[WebViewDownloader] Failed to close fallback webview: {}", e);
                    }

                    if let Some(ref dl_id) = download_id {
                        let _ = self.app_handle.emit("download-error", serde_json::json!({
                            "id": dl_id,
                            "error": "WebView download failed"
                        }));
                    }

                    return WebViewDownloadResult {
                        success: false,
                        download_url: Some(download_url.to_string()),
                        file_path: None,
                        file_size: None,
                        error: Some("WebView download failed".to_string()),
                        cookies: None,
                    };
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        // Timeout
        if let Err(e) = webview.close() {
            log::warn!("[WebViewDownloader] Failed to close fallback webview: {}", e);
        }

        if let Some(ref dl_id) = download_id {
            let _ = self.app_handle.emit("download-error", serde_json::json!({
                "id": dl_id,
                "error": "Download timeout"
            }));
        }

        WebViewDownloadResult {
            success: false,
            download_url: Some(download_url.to_string()),
            file_path: None,
            file_size: None,
            error: Some("Fallback download timeout".to_string()),
            cookies: None,
        }
    }

    fn build_capture_script(&self, event_name: &str, config: &WebViewDownloadConfig) -> String {
        let url_pattern = config.download_url_pattern.as_deref().unwrap_or("");

        format!(
            r#"
            (function() {{
                console.log('[CrackStation] Download capture script injected');

                // Pattern to match download URLs
                const urlPattern = '{}';
                const patternRegex = urlPattern ? new RegExp(urlPattern) : null;

                // Function to check if URL is a download URL
                function isDownloadUrl(url) {{
                    if (!url) return false;
                    if (patternRegex && patternRegex.test(url)) return true;
                    // Common download patterns
                    if (url.includes('/download') || url.includes('dl=') || url.includes('.torrent')) return true;
                    if (url.match(/\.(zip|rar|7z|exe|iso|torrent|bin)(\?|$)/i)) return true;
                    return false;
                }}

                const emitUrl = function(url) {{
                    const full = url.startsWith('/') ? window.location.origin + url : url;
                    console.log('[CrackStation] Emitting download URL:', full);
                    window.__TAURI__.event.emit('{}', {{ url: full }});
                }};

                // Override XMLHttpRequest — intercept both the request URL and the
                // response (HX-Redirect header, responseURL after HTTP redirects,
                // or a URL found in the response body).
                const originalXHROpen = XMLHttpRequest.prototype.open;
                XMLHttpRequest.prototype.open = function(method, url) {{
                    const urlStr = typeof url === 'string' ? url : (url || '').toString();
                    if (isDownloadUrl(urlStr)) {{
                        console.log('[CrackStation] XHR download request intercepted:', urlStr);
                        this.addEventListener('load', function() {{
                            // Priority 1: HTMX redirect header
                            try {{
                                const hxRedirect = this.getResponseHeader('HX-Redirect');
                                if (hxRedirect) {{ emitUrl(hxRedirect); return; }}
                            }} catch(e) {{}}
                            // Priority 2: HTTP redirect — responseURL differs from request URL
                            const absReq = urlStr.startsWith('/') ? window.location.origin + urlStr : urlStr;
                            if (this.responseURL && this.responseURL !== absReq) {{
                                emitUrl(this.responseURL); return;
                            }}
                            // Priority 3: URL embedded in response body
                            try {{
                                const m = this.responseText && this.responseText.match(/(https?:\/\/[^\s"'<>]{{20,}})/);
                                if (m) {{ emitUrl(m[1]); return; }}
                            }} catch(e) {{}}
                            // Fallback: emit the request URL itself so the download manager
                            // can try a direct fetch (may work for simple hosters)
                            emitUrl(urlStr);
                        }});
                    }}
                    return originalXHROpen.apply(this, arguments);
                }};

                // Override fetch to catch fetch-based download URLs
                const originalFetch = window.fetch;
                window.fetch = function(url, options) {{
                    const urlStr = typeof url === 'string' ? url : (url && url.url) || '';
                    if (isDownloadUrl(urlStr)) {{
                        console.log('[CrackStation] Fetch download URL detected:', urlStr);
                        return originalFetch.apply(this, arguments).then(function(resp) {{
                            const hxRedirect = resp.headers.get('HX-Redirect');
                            if (hxRedirect) {{ emitUrl(hxRedirect); }}
                            else if (resp.redirected) {{ emitUrl(resp.url); }}
                            else {{ emitUrl(urlStr); }}
                            return resp;
                        }});
                    }}
                    return originalFetch.apply(this, arguments);
                }};

                // Monitor for clicks on download links
                document.addEventListener('click', function(e) {{
                    const link = e.target.closest('a');
                    if (link && link.href && isDownloadUrl(link.href)) {{
                        console.log('[CrackStation] Click download URL detected:', link.href);
                        window.__TAURI__.event.emit('{}', {{ url: link.href }});
                    }}
                }}, true);

                // Monitor for dynamic content
                const observer = new MutationObserver(function(mutations) {{
                    mutations.forEach(function(mutation) {{
                        mutation.addedNodes.forEach(function(node) {{
                            if (node.querySelectorAll) {{
                                const links = node.querySelectorAll('a[href]');
                                links.forEach(function(link) {{
                                    if (isDownloadUrl(link.href)) {{
                                        console.log('[CrackStation] Dynamic download link found:', link.href);
                                        window.__TAURI__.event.emit('{}', {{ url: link.href }});
                                    }}
                                }});
                            }}
                        }});
                    }});
                }});

                observer.observe(document.body, {{ childList: true, subtree: true }});

                console.log('[CrackStation] Download monitoring active');
            }})();
            "#,
            url_pattern, event_name, event_name, event_name
        )
    }

    async fn wait_for_selector(&self, webview: &tauri::WebviewWindow, selector: &str, timeout_ms: u64) {
        let wait_script = format!(
            r#"
            (function() {{
                return new Promise((resolve) => {{
                    const timeout = setTimeout(() => resolve(false), {});
                    const check = () => {{
                        if (document.querySelector('{}')) {{
                            clearTimeout(timeout);
                            resolve(true);
                        }} else {{
                            requestAnimationFrame(check);
                        }}
                    }};
                    check();
                }});
            }})();
            "#,
            timeout_ms,
            selector.replace('\'', "\\'")
        );

        if let Err(e) = webview.eval(&wait_script) {
            log::warn!("[WebViewDownloader] Wait script failed: {}", e);
        }

        // Also wait in Rust
        let poll_interval = Duration::from_millis(500);
        let max_wait = Duration::from_millis(timeout_ms);
        let start = std::time::Instant::now();

        while start.elapsed() < max_wait {
            tokio::time::sleep(poll_interval).await;
        }
    }
}
