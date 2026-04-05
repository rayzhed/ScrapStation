use crate::config::hosts::{DetectedHost, DownloadMethod, SmartDownloadResult};
use crate::config::paths::{NavigationPath, PathStep, ResolvedLink, ResolutionResult};
use crate::engine::download_manager::DownloadManager;
use crate::engine::host_detector;
use crate::engine::navigator::{Navigator, NavigationContext};
use crate::engine::webview_downloader::{WebViewDownloader};
use crate::engine::SourceLoader;
use crate::settings::UserSettings;
use crate::utils::http_client::HttpClient;
use crate::utils::create_client;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, Manager};

use super::helpers::resolve_link_on_demand;

#[tauri::command]
pub async fn open_url_in_browser(url: String) -> Result<(), String> {
    let url = if url.starts_with("//") { format!("https:{}", url) } else { url };

    // Only allow http/https to prevent protocol-handler abuse and command injection
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(format!("Blocked non-http URL: {}", url));
    }

    #[cfg(target_os = "windows")]
    {
        // Pass an empty string as the window title so `start` cannot mistake
        // the URL for a switch (e.g. "start /b ..." style injection).
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &url])
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn resolve_download_link(url: String, source_id: String) -> Result<String, String> {
    let config = SourceLoader::load_by_id(&source_id)?;
    let cookies = UserSettings::get_cookies(&source_id);

    if let Some(link_config) = &config.link_resolution {
        resolve_link_on_demand(&url, link_config, cookies.as_deref()).await
    } else {
        Ok(url)
    }
}

#[tauri::command]
pub async fn download_file(
    url: String,
    source_id: String,
    resolve_link: Option<bool>,
) -> Result<String, String> {
    let config = SourceLoader::load_by_id(&source_id)?;
    let cookies = UserSettings::get_cookies(&source_id);

    let final_url = if resolve_link.unwrap_or(false) {
        if let Some(link_config) = &config.link_resolution {
            resolve_link_on_demand(&url, link_config, cookies.as_deref()).await?
        } else {
            url
        }
    } else {
        url
    };

    let download_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current dir: {}", e))?
        .join("downloads");

    std::fs::create_dir_all(&download_dir)
        .map_err(|e| format!("Failed to create downloads dir: {}", e))?;

    let filename = final_url
        .split('/')
        .next_back()
        .and_then(|s| s.split('?').next())
        .filter(|s| !s.is_empty())
        .unwrap_or("download.torrent");

    let filename = if filename.contains('.') {
        filename.to_string()
    } else {
        format!("{}.html", filename)
    };

    let file_path = download_dir.join(&filename);

    let client = create_client()?;
    let response = client
        .get(&final_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download file: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    std::fs::write(&file_path, &bytes).map_err(|e| format!("Failed to write file: {}", e))?;

    file_path
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid file path".to_string())
}

/// Detect the host from a URL using the source's hosts configuration
/// If the URL needs resolution (e.g. /ext/ URLs), resolve it first
#[tauri::command]
pub async fn detect_host(
    url: String,
    source_id: String,
    resolve_first: Option<bool>,
) -> Result<DetectedHost, String> {
    let config = SourceLoader::load_by_id(&source_id)?;
    let cookies = UserSettings::get_cookies(&source_id);

    // Optionally resolve the link first to get the actual host URL
    let url_to_detect = if resolve_first.unwrap_or(false) {
        if let Some(link_config) = &config.link_resolution {
            resolve_link_on_demand(&url, link_config, cookies.as_deref())
                .await
                .unwrap_or(url.clone())
        } else {
            url.clone()
        }
    } else {
        url
    };

    Ok(host_detector::detect_host(&url_to_detect, config.hosts.as_ref()))
}

/// Attempt a smart download with automatic host detection and resolution
/// This command spawns the download in the background and returns immediately
#[tauri::command]
pub async fn smart_download(
    app_handle: AppHandle,
    url: String,
    source_id: String,
    filename_hint: Option<String>,
    download_id: Option<String>,
    pre_resolved_url: Option<String>,
    pre_cookies: Option<String>,
    #[allow(non_snake_case)]
    download_dir: Option<String>,
    #[allow(non_snake_case)]
    install_dir: Option<String>,
    tracker: State<'_, Arc<TokioMutex<DownloadTracker>>>,
) -> Result<SmartDownloadResult, String> {
    log::info!("[SmartDownload] Starting for URL: {}", url);
    log::info!("[SmartDownload] Source ID: {}, Download ID: {:?}", source_id, download_id);

    // Get the download ID early
    let dl_id = download_id.clone().unwrap_or_else(|| {
        crate::engine::generate_download_id()
    });

    // Clone what we need for the background task
    let app_handle_clone = app_handle.clone();
    let tracker_clone = tracker.inner().clone();
    let url_clone = url.clone();
    let source_id_clone = source_id.clone();
    let filename_hint_clone = filename_hint.clone();
    let dl_id_clone = dl_id.clone();

    let pre_resolved_url_clone = pre_resolved_url.clone();
    let pre_cookies_clone = pre_cookies.clone();
    let download_dir_clone = download_dir.clone();
    let install_dir_clone = install_dir.clone();

    // Spawn the download in background
    tokio::spawn(async move {
        if let Err(e) = run_smart_download_background(
            app_handle_clone,
            url_clone,
            source_id_clone,
            filename_hint_clone,
            dl_id_clone,
            tracker_clone,
            pre_resolved_url_clone,
            pre_cookies_clone,
            download_dir_clone,
            install_dir_clone,
        ).await {
            log::error!("[SmartDownload] Background download failed: {}", e);
        }
    });

    // Return immediately with pending status
    Ok(SmartDownloadResult {
        success: true, // Indicates the download was started successfully
        file_path: None,
        file_size: None,
        error: None,
        fallback_url: None,
    })
}

/// Background task that runs the actual download
async fn run_smart_download_background(
    app_handle: AppHandle,
    url: String,
    source_id: String,
    filename_hint: Option<String>,
    download_id: String,
    tracker: Arc<TokioMutex<DownloadTracker>>,
    pre_resolved_url: Option<String>,
    pre_cookies: Option<String>,
    download_dir: Option<String>,
    install_dir: Option<String>,
) -> Result<(), String> {
    use crate::engine::download_manager::streaming_download;

    log::info!("[SmartDownloadBg] Starting background download: {}", download_id);

    // Fast path: probe already resolved the URL — skip all resolution and go straight to streaming.
    if let Some(direct_url) = pre_resolved_url {
        log::info!("[SmartDownloadBg] Using pre-resolved URL from probe, skipping resolution: {}", direct_url);
        let source_cookies = UserSettings::get_cookies(&source_id);
        let effective_cookies = pre_cookies.or(source_cookies);
        return perform_streaming_download_bg(
            app_handle,
            direct_url,
            filename_hint,
            download_id,
            tracker,
            effective_cookies,
            download_dir,
            install_dir,
        ).await;
    }

    let config = SourceLoader::load_by_id(&source_id)?;
    let cookies = UserSettings::get_cookies(&source_id);
    log::debug!("[SmartDownloadBg] Has cookies: {}", cookies.is_some());

    // First resolve the link if link_resolution is configured
    let resolved_url = if let Some(link_config) = &config.link_resolution {
        log::info!("[SmartDownloadBg] Link resolution config found, attempting resolution...");

        match resolve_link_on_demand(&url, link_config, cookies.as_deref()).await {
            Ok(resolved) => {
                log::info!("[SmartDownloadBg] Resolution result: {}", resolved);
                resolved
            }
            Err(e) => {
                log::error!("[SmartDownloadBg] Resolution error: {}", e);
                url.clone()
            }
        }
    } else {
        url.clone()
    };

    // Normalize protocol-relative URLs (//host.com/...) to https://
    let resolved_url = if resolved_url.starts_with("//") {
        format!("https:{}", resolved_url)
    } else {
        resolved_url
    };

    log::info!("[SmartDownloadBg] Final URL to download: {}", resolved_url);

    // Detect host and check download method
    let (_detected, host_config) = host_detector::detect_host_with_config(&resolved_url, config.hosts.as_ref());

    // Check if this host requires WebView download
    if let Some(hc) = &host_config {
        if hc.download_method == DownloadMethod::Webview {
            log::info!("[SmartDownloadBg] Host requires WebView download");

            // Mark this download as non-resumable
            {
                let tracker_guard = tracker.lock().await;
                let _ = tracker_guard.mark_non_resumable(&download_id).await;
            }

            if let Some(webview_config) = &hc.webview_config {
                let downloader = WebViewDownloader::new(app_handle.clone());
                let result = downloader.get_download_url(&resolved_url, webview_config, Some(download_id.clone()), false).await;

                if result.success {
                    if let Some(file_path) = result.file_path {
                        log::info!("[SmartDownloadBg] WebView downloaded file to: {}", file_path);
                        let tracker_guard = tracker.lock().await;
                        let _ = tracker_guard.complete_download(&download_id, &file_path, result.file_size.unwrap_or(0)).await;
                        return Ok(());
                    }

                    // WebView captured URL - use streaming download
                    if let Some(direct_url) = result.download_url {
                        log::info!("[SmartDownloadBg] WebView captured URL, streaming: {}", direct_url);
                        return perform_streaming_download_bg(
                            app_handle,
                            direct_url,
                            filename_hint,
                            download_id,
                            tracker,
                            cookies.clone(),
                            download_dir,
                            install_dir,
                        ).await;
                    }
                }

                // WebView did not succeed — distinguish paused/cancelled from real errors
                let error_msg = result.error.unwrap_or_else(|| "WebView download failed".to_string());

                if error_msg == "Download paused" {
                    // Already marked as paused by the webview downloader — not a real error
                    return Ok(());
                }
                if error_msg == "Download cancelled" {
                    // Already marked as cancelled — not a real error
                    return Ok(());
                }

                let tracker_guard = tracker.lock().await;
                let _ = tracker_guard.fail_download(&download_id, &error_msg).await;

                let _ = app_handle.emit("download-error", serde_json::json!({
                    "id": download_id,
                    "error": error_msg
                }));

                return Err(error_msg);
            }
        }
    }

    // Standard streaming download
    perform_streaming_download_bg(
        app_handle,
        resolved_url,
        filename_hint,
        download_id,
        tracker,
        cookies,
        download_dir,
        install_dir,
    ).await
}

/// Background streaming download helper (takes Arc instead of State)
async fn perform_streaming_download_bg(
    app_handle: AppHandle,
    url: String,
    filename_hint: Option<String>,
    download_id: String,
    tracker: Arc<TokioMutex<DownloadTracker>>,
    cookies: Option<String>,
    download_dir: Option<String>,
    install_dir: Option<String>,
) -> Result<(), String> {
    use crate::engine::download_manager::streaming_download;

    let download_folder = if let Some(ref dir) = download_dir {
        let p = std::path::PathBuf::from(dir);
        if !p.exists() { let _ = std::fs::create_dir_all(&p); }
        p
    } else {
        get_download_folder(&app_handle)
    };

    // Persist custom dirs so resume / resolve_file_path / extraction know where to look
    {
        let tracker_guard = tracker.lock().await;
        if let Some(ref dir) = download_dir {
            let _ = tracker_guard.set_download_dir(&download_id, dir).await;
        }
        if let Some(ref dir) = install_dir {
            let _ = tracker_guard.set_install_dir(&download_id, dir).await;
        }
    }

    // Get signals from tracker
    let signals = {
        let tracker_guard = tracker.lock().await;
        tracker_guard.get_signals()
    };

    // Update status to downloading AND save the resolved URL and cookies for resume
    {
        let tracker_guard = tracker.lock().await;
        let _ = tracker_guard.update_status(&download_id, DownloadStatus::Downloading).await;
        // Save the URL we're actually downloading from (for resume to work)
        let _ = tracker_guard.update_resolved_url(&download_id, &url).await;
        // Save cookies if available (for authenticated resume)
        if let Some(ref c) = cookies {
            let _ = tracker_guard.update_cookies(&download_id, c).await;
        }
        // Mark as resumable since streaming_download supports pause/resume
        let _ = tracker_guard.mark_resumable(&download_id).await;
    }

    // Create progress callback
    let app_handle_clone = app_handle.clone();
    let dl_id_clone = download_id.clone();
    let tracker_clone = tracker.clone();

    let progress_callback = Box::new(move |downloaded: u64, total: u64, speed: u64| {
        // Emit progress event to frontend
        let _ = app_handle_clone.emit("download-progress", serde_json::json!({
            "id": dl_id_clone,
            "downloaded_bytes": downloaded,
            "total_bytes": total,
            "speed": speed
        }));

        // Update tracker (fire-and-forget)
        let tracker_inner = tracker_clone.clone();
        let id = dl_id_clone.clone();
        tokio::spawn(async move {
            if let Ok(t) = tracker_inner.try_lock() {
                let _ = t.update_progress(&id, downloaded, total).await;
            }
        });
    });

    // Perform streaming download
    match streaming_download(
        &url,
        filename_hint,
        download_folder,
        download_id.clone(),
        signals,
        Some(progress_callback),
        cookies,
    ).await {
        Ok(result) => {
            // Update tracker with actual filename and completion
            {
                let tracker_guard = tracker.lock().await;
                let _ = tracker_guard.update_filename(&download_id, &result.actual_filename).await;
                let _ = tracker_guard.update_file_path(&download_id, &result.file_path).await;
                let _ = tracker_guard.complete_download(&download_id, &result.file_path, result.file_size).await;
            }

            // Emit completion event
            let _ = app_handle.emit("download-complete", serde_json::json!({
                "id": download_id,
                "file_path": result.file_path
            }));

            log::info!("[SmartDownloadBg] Download completed: {} -> {}", download_id, result.file_path);

            // Auto-extract if this download is linked to a library game
            maybe_auto_extract(app_handle, &download_id, &result.file_path).await;

            Ok(())
        }
        Err(e) => {
            use crate::engine::download_manager::StreamingDownloadError;

            match e {
                StreamingDownloadError::Paused(info) => {
                    log::info!("[SmartDownloadBg] Download paused: {} at {} / {} bytes",
                        download_id, info.downloaded_bytes, info.total_bytes);
                    // Save all progress information for resume
                    let tracker_guard = tracker.lock().await;
                    let _ = tracker_guard.update_filename(&download_id, &info.actual_filename).await;
                    let _ = tracker_guard.update_file_path(&download_id, &info.file_path).await;
                    let _ = tracker_guard.pause_download_with_progress(&download_id, info.downloaded_bytes, info.total_bytes).await;
                    return Ok(()); // Not an error, just paused
                }
                StreamingDownloadError::Cancelled => {
                    log::info!("[SmartDownloadBg] Download cancelled: {}", download_id);
                    let tracker_guard = tracker.lock().await;
                    let _ = tracker_guard.update_status(&download_id, DownloadStatus::Cancelled).await;
                    return Ok(()); // Already marked as cancelled
                }
                StreamingDownloadError::Error(msg) => {
                    // Regular error
                    let tracker_guard = tracker.lock().await;
                    let _ = tracker_guard.fail_download(&download_id, &msg).await;

                    let _ = app_handle.emit("download-error", serde_json::json!({
                        "id": download_id,
                        "error": &msg
                    }));

                    log::error!("[SmartDownloadBg] Download failed: {} - {}", download_id, msg);
                    Err(msg)
                }
            }
        }
    }
}

/// Navigate and resolve a link using the new path-based system
/// This is the new flexible link resolution that supports tree navigation
#[tauri::command]
pub async fn navigate_link(
    url: String,
    source_id: String,
    path_name: Option<String>,
    inline_path: Option<Vec<PathStep>>,
) -> Result<ResolutionResult, String> {
    use std::time::Instant;

    log::info!("[NavigateLink] Starting navigation for URL: {}", url);
    let start = Instant::now();

    let config = SourceLoader::load_by_id(&source_id)?;
    let cookies = UserSettings::get_cookies(&source_id);

    // Load named paths from source config
    let paths: HashMap<String, NavigationPath> = config
        .paths
        .clone()
        .unwrap_or_default();

    // Determine which path to use
    let path = if let Some(steps) = inline_path {
        // Inline path provided
        NavigationPath::simple(steps)
    } else if let Some(name) = path_name {
        // Named path
        paths
            .get(&name)
            .cloned()
            .ok_or(format!("Path '{}' not found in source config", name))?
    } else {
        // Default: simple fetch and return
        NavigationPath::simple(vec![
            PathStep::Fetch(crate::config::paths::FetchStep::default()),
            PathStep::Return(crate::config::paths::ReturnStep {
                current: true,
                ..Default::default()
            }),
        ])
    };

    // Create navigation context
    let mut ctx = NavigationContext::new(url, source_id, cookies, paths);
    ctx = ctx.with_timeout(path.timeout_ms).with_max_depth(path.max_depth);

    // Execute navigation
    let navigator = Navigator::new();
    let links = navigator.execute(&path, &mut ctx).await?;

    let duration_ms = start.elapsed().as_millis() as u64;

    log::info!(
        "[NavigateLink] Completed in {}ms with {} links",
        duration_ms,
        links.len()
    );

    Ok(ResolutionResult {
        links,
        groups: None,
        warnings: ctx.warnings,
        duration_ms,
    })
}

/// Resolve a button's link using its configured path or resolver
#[tauri::command]
pub async fn resolve_button_link(
    url: String,
    source_id: String,
    button_config: serde_json::Value,
) -> Result<ResolutionResult, String> {
    use std::time::Instant;
    use crate::types::detail_section::DownloadButtonConfig;

    log::info!("[ResolveButtonLink] Starting for URL: {}", url);
    let start = Instant::now();

    let config = SourceLoader::load_by_id(&source_id)?;
    let cookies = UserSettings::get_cookies(&source_id);

    // Parse button config
    let button: DownloadButtonConfig = serde_json::from_value(button_config)
        .map_err(|e| format!("Invalid button config: {}", e))?;

    // Load named paths from source config
    let paths: HashMap<String, NavigationPath> = config
        .paths
        .clone()
        .unwrap_or_default();

    // Also load resolvers from link_resolution (legacy compatibility)
    let mut all_paths = paths;
    if let Some(_link_res) = &config.link_resolution {
        if let Some(resolvers) = config.extra.get("resolvers") {
            if let Ok(r) = serde_json::from_value::<HashMap<String, NavigationPath>>(resolvers.clone()) {
                all_paths.extend(r);
            }
        }
    }

    // Determine path to use
    let path = if let Some(steps) = button.path {
        // Inline path in button config
        NavigationPath::simple(steps)
    } else if let Some(resolver_name) = button.resolver {
        // Named resolver
        all_paths
            .get(&resolver_name)
            .cloned()
            .ok_or(format!("Resolver '{}' not found", resolver_name))?
    } else if let Some(resolution) = button.resolution {
        // Resolution config with matching rules
        determine_path_from_resolution(&url, &resolution, &all_paths)?
    } else if button.smart_download.unwrap_or(false) {
        // Smart download: auto-detect host
        return smart_download_as_resolution(url, source_id, cookies).await;
    } else {
        // Default: just return the URL
        NavigationPath::simple(vec![PathStep::Return(crate::config::paths::ReturnStep {
            value: Some(url.clone()),
            ..Default::default()
        })])
    };

    // Create context and execute
    let mut ctx = NavigationContext::new(url, source_id, cookies, all_paths);
    ctx = ctx.with_timeout(path.timeout_ms).with_max_depth(path.max_depth);

    let navigator = Navigator::new();
    let links = navigator.execute(&path, &mut ctx).await?;

    let duration_ms = start.elapsed().as_millis() as u64;

    log::info!(
        "[ResolveButtonLink] Completed in {}ms with {} links",
        duration_ms,
        links.len()
    );

    Ok(ResolutionResult {
        links,
        groups: None,
        warnings: ctx.warnings,
        duration_ms,
    })
}

fn determine_path_from_resolution(
    url: &str,
    resolution: &crate::types::detail_section::ButtonResolutionConfig,
    paths: &HashMap<String, NavigationPath>,
) -> Result<NavigationPath, String> {
    use crate::types::detail_section::MatchCondition;

    // Check match rules
    if let Some(rules) = &resolution.match_rules {
        for rule in rules {
            let matches = match &rule.when {
                MatchCondition::Contains(s) => url.contains(s),
                MatchCondition::Matches(pattern) => {
                    regex::Regex::new(pattern)
                        .map(|re| re.is_match(url))
                        .unwrap_or(false)
                }
                MatchCondition::StartsWith(s) => url.starts_with(s),
                MatchCondition::EndsWith(s) => url.ends_with(s),
                MatchCondition::HostEquals(host) => url::Url::parse(url)
                    .ok()
                    .and_then(|u| u.host_str().map(|h| h == host))
                    .unwrap_or(false),
                MatchCondition::HostContains(s) => url::Url::parse(url)
                    .ok()
                    .and_then(|u| u.host_str().map(|h| h.contains(s)))
                    .unwrap_or(false),
                MatchCondition::Always => true,
            };

            if matches {
                if let Some(resolver_name) = &rule.resolver {
                    return paths
                        .get(resolver_name)
                        .cloned()
                        .ok_or(format!("Resolver '{}' not found", resolver_name));
                }
                if let Some(path_ref) = &rule.path {
                    return path_ref_to_path(path_ref, paths);
                }
            }
        }
    }

    // Use default resolver
    if let Some(default_name) = &resolution.default {
        return paths
            .get(default_name)
            .cloned()
            .ok_or(format!("Default resolver '{}' not found", default_name));
    }

    // Use inline steps
    if let Some(steps) = &resolution.steps {
        return Ok(NavigationPath::simple(steps.clone()));
    }

    Err("No matching path found for URL".to_string())
}

fn path_ref_to_path(
    path_ref: &crate::config::paths::PathOrRef,
    paths: &HashMap<String, NavigationPath>,
) -> Result<NavigationPath, String> {
    match path_ref {
        crate::config::paths::PathOrRef::Inline(steps) => Ok(NavigationPath::simple(steps.clone())),
        crate::config::paths::PathOrRef::InlineFull(path) => Ok((*path.clone()).clone()),
        crate::config::paths::PathOrRef::Reference(use_step) => {
            let name = use_step.path_ref.trim_start_matches("paths.");
            paths
                .get(name)
                .cloned()
                .ok_or(format!("Path '{}' not found", name))
        }
    }
}

async fn smart_download_as_resolution(
    url: String,
    source_id: String,
    cookies: Option<String>,
) -> Result<ResolutionResult, String> {
    use std::time::Instant;

    let start = Instant::now();
    let config = SourceLoader::load_by_id(&source_id)?;

    // First resolve if needed
    let resolved_url = if let Some(link_config) = &config.link_resolution {
        resolve_link_on_demand(&url, link_config, cookies.as_deref())
            .await
            .unwrap_or(url.clone())
    } else {
        url.clone()
    };

    // Detect host
    let detected = host_detector::detect_host(&resolved_url, config.hosts.as_ref());

    let link = ResolvedLink {
        url: resolved_url,
        label: None,
        host: Some(detected.label),
        size: None,
        browser_only: !detected.supports_direct_download,
        browser_only_reason: detected.browser_only_reason,
        metadata: HashMap::new(),
        resolution_path: vec![url],
    };

    Ok(ResolutionResult {
        links: vec![link],
        groups: None,
        warnings: vec![],
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

// ============== DOWNLOAD TRACKER COMMANDS ==============

use crate::engine::{DownloadTracker, DownloadEntry, DownloadStatus, get_download_folder};
use tauri::State;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

/// Register a new download in the tracker
#[tauri::command]
pub async fn register_download(
    id: String,
    game_title: String,
    file_name: String,
    url: String,
    source_id: String,
    host_label: String,
    host_color: String,
    tracker: State<'_, Arc<TokioMutex<DownloadTracker>>>,
) -> Result<(), String> {
    let entry = DownloadEntry {
        id,
        game_title,
        file_name,
        url,
        resolved_url: None,
        cookies: None,
        source_id,
        host_label,
        host_color,
        status: DownloadStatus::Pending,
        downloaded_bytes: 0,
        total_bytes: 0,
        file_path: None,
        download_dir: None,
        install_dir: None,
        error: None,
        started_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        completed_at: None,
        is_resumable: true,
    };

    let tracker = tracker.lock().await;
    tracker.add_download(entry).await
}

/// Update download status
#[tauri::command]
pub async fn update_download_status(
    id: String,
    status: String,
    error: Option<String>,
    file_path: Option<String>,
    tracker: State<'_, Arc<TokioMutex<DownloadTracker>>>,
) -> Result<(), String> {
    let tracker = tracker.lock().await;

    let download_status = match status.as_str() {
        "queued" => DownloadStatus::Queued,
        "pending" => DownloadStatus::Pending,
        "downloading" => DownloadStatus::Downloading,
        "paused" => DownloadStatus::Paused,
        "completed" => DownloadStatus::Completed,
        "failed" => DownloadStatus::Failed,
        "cancelled" => DownloadStatus::Cancelled,
        _ => return Err(format!("Unknown status: {}", status)),
    };

    if download_status == DownloadStatus::Completed {
        if let Some(path) = file_path {
            tracker.complete_download(&id, &path, 0).await?;
        } else {
            tracker.update_status(&id, download_status).await?;
        }
    } else if download_status == DownloadStatus::Failed {
        tracker.fail_download(&id, &error.unwrap_or_else(|| "Unknown error".to_string())).await?;
    } else {
        tracker.update_status(&id, download_status).await?;
    }

    Ok(())
}

/// Get all downloads from the tracker.
/// Resolves relative file_path values to absolute before sending to the frontend,
/// so that open_file_location and similar calls always receive absolute paths.
#[tauri::command]
pub async fn get_downloads(
    app_handle: AppHandle,
    tracker: State<'_, Arc<TokioMutex<DownloadTracker>>>,
) -> Result<Vec<DownloadEntry>, String> {
    let tracker = tracker.lock().await;
    let mut entries = tracker.get_all_downloads().await;
    for entry in &mut entries {
        if let Some(fp) = &entry.file_path {
            let path = std::path::Path::new(fp);
            if !path.is_absolute() {
                entry.file_path = Some(
                    get_download_folder(&app_handle).join(fp).to_string_lossy().to_string()
                );
            }
        }
    }
    Ok(entries)
}

/// Pause a download
#[tauri::command]
pub async fn pause_download(
    id: String,
    tracker: State<'_, Arc<TokioMutex<DownloadTracker>>>,
) -> Result<(), String> {
    let tracker = tracker.lock().await;
    tracker.pause_download(&id).await
}

/// Resume a paused download
#[tauri::command]
pub async fn resume_download(
    app_handle: AppHandle,
    id: String,
    tracker: State<'_, Arc<TokioMutex<DownloadTracker>>>,
) -> Result<(), String> {
    let entry = {
        let tracker = tracker.lock().await;
        tracker.resume_download(&id).await?
    };

    // Re-start the download in background
    let tracker_clone = tracker.inner().clone();
    tokio::spawn(async move {
        if let Err(e) = resume_download_task(app_handle, entry, tracker_clone).await {
            log::error!("[ResumeDownload] Failed: {}", e);
        }
    });

    Ok(())
}

/// Cancel a download
#[tauri::command]
pub async fn cancel_download(
    id: String,
    tracker: State<'_, Arc<TokioMutex<DownloadTracker>>>,
) -> Result<(), String> {
    let tracker = tracker.lock().await;
    tracker.cancel_download(&id).await
}

/// Remove a download from the list
#[tauri::command]
pub async fn remove_download(
    id: String,
    tracker: State<'_, Arc<TokioMutex<DownloadTracker>>>,
) -> Result<(), String> {
    let tracker = tracker.lock().await;
    tracker.remove_download(&id).await
}

/// Clear all completed/failed downloads
#[tauri::command]
pub async fn clear_finished_downloads(
    tracker: State<'_, Arc<TokioMutex<DownloadTracker>>>,
) -> Result<(), String> {
    let tracker = tracker.lock().await;
    tracker.clear_finished().await
}

/// Get the download folder path
#[tauri::command]
pub async fn get_download_folder_path(
    app_handle: AppHandle,
) -> Result<String, String> {
    Ok(get_download_folder(&app_handle).to_string_lossy().to_string())
}

/// Open the download folder in file explorer
#[tauri::command]
pub async fn open_download_folder(
    app_handle: AppHandle,
) -> Result<(), String> {
    let folder = get_download_folder(&app_handle);

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

/// Open a specific file's location in file explorer
#[tauri::command]
pub async fn open_file_location(
    file_path: String,
) -> Result<(), String> {
    let _path = std::path::Path::new(&file_path);

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &file_path])
            .spawn()
            .map_err(|e| format!("Failed to open location: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &file_path])
            .spawn()
            .map_err(|e| format!("Failed to open location: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(parent) = path.parent() {
            std::process::Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(|e| format!("Failed to open location: {}", e))?;
        }
    }

    Ok(())
}

/// Helper to resume a download task
async fn resume_download_task(
    app_handle: AppHandle,
    entry: DownloadEntry,
    tracker: Arc<TokioMutex<DownloadTracker>>,
) -> Result<(), String> {
    #[allow(unused_imports)]
    use crate::engine::webview_downloader::WebViewDownloader;
    use futures_util::StreamExt;
    use reqwest::header::{HeaderMap, HeaderValue, RANGE, COOKIE};
    use std::io::Write;

    log::info!("[ResumeDownload] Resuming download: {} from {} bytes", entry.id, entry.downloaded_bytes);

    // CRITICAL: Use the resolved URL if available (the actual download link)
    // The original URL might be a page (gofile.io/d/xxx) not a direct download
    let download_url = entry.resolved_url.as_ref().unwrap_or(&entry.url);
    log::info!("[ResumeDownload] Using URL: {}", download_url);

    if entry.resolved_url.is_none() {
        log::warn!("[ResumeDownload] No resolved URL - this might not work for hosts like gofile!");
    }

    let download_folder = get_download_folder(&app_handle);
    // Resolve saved file_path (may be relative or legacy absolute), fallback to filename
    let destination = crate::engine::download_tracker::resolve_file_path(&entry, &app_handle)
        .unwrap_or_else(|| download_folder.join(&entry.file_name));

    log::info!("[ResumeDownload] Destination: {:?}", destination);

    // Update status to downloading
    {
        let tracker = tracker.lock().await;
        tracker.update_status(&entry.id, DownloadStatus::Downloading).await?;
    }

    // Try to resume with Range header if we have partial data
    let mut headers = HeaderMap::new();
    headers.insert("User-Agent", HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"));

    // Add cookies if available (from saved download state or settings)
    if let Some(ref cookies) = entry.cookies {
        if let Ok(cookie_value) = HeaderValue::from_str(cookies) {
            headers.insert(COOKIE, cookie_value);
            log::info!("[ResumeDownload] Using saved cookies");
        }
    } else if let Some(cookies) = UserSettings::get_cookies(&entry.source_id) {
        if let Ok(cookie_value) = HeaderValue::from_str(&cookies) {
            headers.insert(COOKIE, cookie_value);
            log::info!("[ResumeDownload] Using cookies from settings");
        }
    }

    let resume_from = if entry.downloaded_bytes > 0 && entry.is_resumable && destination.exists() {
        // Verify file size matches expected
        let file_size = std::fs::metadata(&destination).map(|m| m.len()).unwrap_or(0);
        if file_size == entry.downloaded_bytes {
            headers.insert(RANGE, HeaderValue::from_str(&format!("bytes={}-", entry.downloaded_bytes))
                .map_err(|e| e.to_string())?);
            log::info!("[ResumeDownload] Resuming from byte {} (file size verified)", entry.downloaded_bytes);
            entry.downloaded_bytes
        } else {
            log::warn!("[ResumeDownload] File size mismatch: expected {}, got {}. Starting fresh.", entry.downloaded_bytes, file_size);
            0
        }
    } else {
        log::info!("[ResumeDownload] Starting fresh (no partial data or file doesn't exist)");
        0
    };

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.get(download_url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    // Check if server supports range requests
    let (is_partial, total_size) = if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
        // Server supports range, we're resuming
        let total = response.content_length().map(|l| l + resume_from).unwrap_or(entry.total_bytes);
        log::info!("[ResumeDownload] Server returned 206, resuming. Remaining: {} bytes", total - resume_from);
        (true, total)
    } else if response.status().is_success() {
        let content_length = response.content_length().unwrap_or(0);

        // Check if we're getting an error page instead of the actual file
        // If we expected a large file but got a tiny response, something is wrong
        if entry.total_bytes > 0 && content_length > 0 && content_length < 100_000 && entry.total_bytes > 1_000_000 {
            log::error!("[ResumeDownload] Suspicious response size: {} bytes (expected ~{} bytes). Likely an error page.",
                content_length, entry.total_bytes);
            let tracker = tracker.lock().await;
            tracker.fail_download(&entry.id, "Resume failed - authentication may have expired").await?;
            return Err("Resume failed - authentication may have expired. Try starting a new download.".to_string());
        }

        log::info!("[ResumeDownload] Server returned 200 (no range support), content-length: {} bytes", content_length);
        (false, content_length)
    } else {
        let tracker = tracker.lock().await;
        tracker.fail_download(&entry.id, &format!("HTTP error: {}", response.status())).await?;
        return Err(format!("HTTP error: {}", response.status()));
    };

    // Open file for writing (append if resuming, create if not)
    let mut file = if is_partial && resume_from > 0 {
        std::fs::OpenOptions::new()
            .append(true)
            .open(&destination)
            .map_err(|e| format!("Failed to open file: {}", e))?
    } else {
        std::fs::File::create(&destination)
            .map_err(|e| format!("Failed to create file: {}", e))?
    };

    let mut downloaded = if is_partial { resume_from } else { 0 };
    let mut stream = response.bytes_stream();
    let mut last_emit = std::time::Instant::now();
    let mut last_bytes = downloaded;

    while let Some(chunk_result) = stream.next().await {
        // Check for pause/cancel signals
        let signal = {
            let tracker = tracker.lock().await;
            tracker.check_signal(&entry.id).await
        };

        match signal {
            crate::engine::DownloadSignal::Pause => {
                log::info!("[ResumeDownload] Download paused: {}", entry.id);
                let tracker = tracker.lock().await;
                tracker.update_progress(&entry.id, downloaded, total_size).await?;
                tracker.update_status(&entry.id, DownloadStatus::Paused).await?;
                return Ok(());
            }
            crate::engine::DownloadSignal::Cancel => {
                log::info!("[ResumeDownload] Download cancelled: {}", entry.id);
                // Don't delete file - user might want to resume later
                return Ok(());
            }
            _ => {}
        }

        let chunk = chunk_result.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk).map_err(|e| format!("Write error: {}", e))?;
        downloaded += chunk.len() as u64;

        // Emit progress every 250ms
        if last_emit.elapsed() >= std::time::Duration::from_millis(250) {
            let speed = ((downloaded - last_bytes) as f64 / last_emit.elapsed().as_secs_f64()) as u64;

            let _ = app_handle.emit("download-progress", serde_json::json!({
                "id": entry.id,
                "downloaded_bytes": downloaded,
                "total_bytes": total_size,
                "speed": speed
            }));

            let tracker = tracker.lock().await;
            tracker.update_progress(&entry.id, downloaded, total_size).await?;

            last_emit = std::time::Instant::now();
            last_bytes = downloaded;
        }
    }

    file.flush().map_err(|e| format!("Flush error: {}", e))?;

    // Complete the download
    let file_path_str = destination.to_string_lossy().to_string();
    {
        let tracker = tracker.lock().await;
        tracker.complete_download(&entry.id, &file_path_str, downloaded).await?;
    }

    let _ = app_handle.emit("download-complete", serde_json::json!({
        "id": entry.id,
        "file_path": &file_path_str
    }));

    log::info!("[ResumeDownload] Download completed: {} ({} bytes)", entry.id, downloaded);

    // Auto-extract if this download is linked to a library game
    maybe_auto_extract(app_handle, &entry.id, &file_path_str).await;

    Ok(())
}

// ============== AUTO-EXTRACTION ==============

/// Called when any single download completes.
/// Finds the linked library game, checks whether ALL its downloads are now done,
/// and — if so — triggers extraction with every part at once.
async fn maybe_auto_extract(app_handle: AppHandle, download_id: &str, _file_path: &str) {
    use crate::engine::LibraryTracker;

    let lt_arc = match app_handle.try_state::<Arc<TokioMutex<LibraryTracker>>>() {
        Some(s) => (*s).clone(),
        None => return,
    };
    let dt_arc = match app_handle.try_state::<Arc<TokioMutex<DownloadTracker>>>() {
        Some(s) => (*s).clone(),
        None => return,
    };

    let game = {
        let lt = lt_arc.lock().await;
        lt.find_by_download_id(download_id).await
    };

    let game = match game {
        Some(g) => g,
        None => return,
    };

    // Collect the file paths of all downloads linked to this game.
    // If some are not yet complete (multi-part in-progress), bail out —
    // the next part's completion will call us again.
    let (file_paths, custom_install_dir): (Vec<String>, Option<String>) = {
        let dt = dt_arc.lock().await;
        let entries = dt.get_entries_by_ids(&game.download_ids).await;

        // All known entries must be completed
        let all_complete = !entries.is_empty()
            && entries.iter().all(|e| e.status == DownloadStatus::Completed);

        if !all_complete {
            log::info!(
                "[AutoExtract] Not all parts complete yet for game '{}' — waiting.",
                game.title
            );
            return;
        }

        // Grab custom install dir from the triggering download (first entry that has one)
        let install_dir = entries.iter().find_map(|e| e.install_dir.clone());

        let paths = entries
            .into_iter()
            .filter_map(|e| {
                let resolved = crate::engine::download_tracker::resolve_file_path(&e, &app_handle)?;
                if !resolved.exists() { return None; }
                Some(resolved.to_string_lossy().to_string())
            })
            .collect();

        (paths, install_dir)
    };

    if file_paths.is_empty() {
        log::error!("[AutoExtract] No archive files found for game '{}' — emitting error.", game.title);
        {
            let lt = lt_arc.lock().await;
            use crate::engine::library_tracker::LibraryGameStatus;
            let _ = lt.update_status(&game.id, LibraryGameStatus::Corrupted).await;
        }
        let _ = app_handle.emit("extraction-error", serde_json::json!({
            "gameId": game.id,
            "error": "No archive files were found after download completed."
        }));
        return;
    }

    log::info!(
        "[AutoExtract] All {} part(s) ready for '{}' — starting extraction.",
        file_paths.len(),
        game.title
    );

    let app_clone = app_handle.clone();
    let game_id = game.id.clone();
    tokio::spawn(async move {
        trigger_auto_extraction(app_clone, game_id, file_paths, lt_arc, custom_install_dir).await;
    });
}

/// Run archive extraction for a library game after all its downloads complete.
async fn trigger_auto_extraction(
    app_handle: AppHandle,
    game_id: String,
    file_paths: Vec<String>,
    library_tracker: Arc<TokioMutex<crate::engine::LibraryTracker>>,
    custom_install_dir: Option<String>,
) {
    use crate::engine::archive_extractor::{ArchiveExtractor, delete_archives};
    use crate::engine::executable_detector::ExecutableDetector;
    use crate::engine::library_tracker::LibraryGameStatus;
    use std::path::PathBuf;

    // Fetch game metadata (install path + password)
    let game = {
        let lt = library_tracker.lock().await;
        match lt.get_game(&game_id).await {
            Some(g) => g,
            None => {
                log::error!("[AutoExtract] Game not found in library: {}", game_id);
                let _ = app_handle.emit("extraction-error", serde_json::json!({
                    "gameId": game_id,
                    "error": "Game not found in library — cannot extract."
                }));
                return;
            }
        }
    };

    // Mark as extracting
    {
        let lt = library_tracker.lock().await;
        let _ = lt.update_status(&game_id, LibraryGameStatus::Extracting).await;
    }

    let paths: Vec<PathBuf> = file_paths.iter().map(PathBuf::from).collect();

    // Determine destination: custom install dir overrides the library default
    let destination = if let Some(ref dir) = custom_install_dir {
        let slug = crate::engine::library_tracker::generate_game_slug(&game.title);
        PathBuf::from(dir).join(&slug).join("game")
    } else {
        crate::engine::library_tracker::resolve_install_path(&game, &app_handle)
    };

    let password = game.archive_password.clone();

    log::info!(
        "[AutoExtract] Extracting {} file(s) -> {:?} (password: {})",
        paths.len(),
        destination,
        if password.is_some() { "yes" } else { "no" }
    );

    let extractor = ArchiveExtractor::new(app_handle.clone());
    match extractor.extract(paths.clone(), destination.clone(), password, game_id.clone()).await {
        Ok(result) => {
            log::info!("[AutoExtract] Done: {} files, {} bytes", result.files_extracted, result.total_size);

            if result.files_extracted == 0 {
                log::error!("[AutoExtract] Extraction reported success but 0 files extracted.");
                let lt = library_tracker.lock().await;
                let _ = lt.update_status(&game_id, LibraryGameStatus::Corrupted).await;
                let _ = app_handle.emit("extraction-error", serde_json::json!({
                    "gameId": game_id,
                    "error": "disk_space: Not enough disk space — no files were extracted."
                }));
                return;
            }

            let executables = ExecutableDetector::detect_executables(&destination, &game.title);
            let install_size = ExecutableDetector::calculate_directory_size(&destination);

            {
                let lt = library_tracker.lock().await;
                // If a custom install dir was used, persist the absolute path in the library
                // so launch/open-folder resolve to the right place.
                if custom_install_dir.is_some() {
                    let abs_path = destination.to_string_lossy().to_string();
                    let _ = lt.update_install_path(&game_id, &abs_path).await;
                }
                let _ = lt.set_executables(&game_id, executables).await;
                let _ = lt.set_install_size(&game_id, install_size).await;
                let _ = lt.update_status(&game_id, LibraryGameStatus::Ready).await;
            }

            if let Err(e) = delete_archives(&paths).await {
                log::warn!("[AutoExtract] Failed to delete archives: {}", e);
            }

            let _ = app_handle.emit("extraction-complete", serde_json::json!({
                "gameId": game_id,
                "success": true
            }));
        }
        Err(e) => {
            log::error!("[AutoExtract] Extraction failed for {}: {}", game_id, e);

            let err_str = e.to_string();
            let tagged = if err_str.to_lowercase().contains("no space left")
                || err_str.to_lowercase().contains("disk is full")
                || err_str.to_lowercase().contains("not enough space")
                || err_str.to_lowercase().contains("insufficient")
                || err_str.to_lowercase().contains("write error")
            {
                format!("disk_space: {}", err_str)
            } else {
                err_str
            };

            // Mark as corrupted so the Extract button can find and retry it
            {
                let lt = library_tracker.lock().await;
                let _ = lt.update_status(&game_id, LibraryGameStatus::Corrupted).await;
            }

            let _ = app_handle.emit("extraction-error", serde_json::json!({
                "gameId": game_id,
                "error": tagged
            }));
        }
    }
}

// ============================================================================
// INSTALL PREFLIGHT — disk space check before downloading
// ============================================================================

/// Result returned by probe_download_size.
/// Contains the file size AND the fully-resolved direct URL + cookies so the
/// real download can skip all resolution steps and start immediately.
#[derive(serde::Serialize)]
pub struct ProbeResult {
    /// Archive size in bytes (0 = unknown)
    pub size: u64,
    /// Fully resolved direct download URL (after source + host resolution)
    pub resolved_url: String,
    /// Cookie header string extracted from the webview session (webview hosts only)
    pub cookies: Option<String>,
}

#[derive(serde::Serialize)]
pub struct InstallPreflight {
    /// Compressed archive size from Content-Length header (0 = unknown)
    pub download_size_bytes: u64,
    /// Where the archive will be downloaded to
    pub download_path: String,
    /// Where the game will be installed on this machine
    pub install_path: String,
    /// Free bytes available on the download drive (0 = could not determine)
    pub available_bytes: u64,
}

/// Try to get the full file size of `url` without downloading the body.
///
/// Strategy:
/// 1. HEAD request — works for direct servers that honour it.
/// 2. GET with `Range: bytes=0-0` — returns 206 Partial Content whose
///    `Content-Range: bytes 0-0/<total>` header gives the full size.
///    Most file-hosts (GoFile, PixelDrain, Rootz, …) support this even when
///    they reject plain HEAD requests.
async fn head_content_length(url: &str, source_id: &str) -> u64 {
    let client = match create_client() {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let cookies = UserSettings::get_cookies(source_id);

    // ── 1. HEAD ──────────────────────────────────────────────────────────────
    {
        let mut req = client
            .head(url)
            .header("User-Agent", crate::constants::USER_AGENT);
        if let Some(ref c) = cookies {
            req = req.header("Cookie", c.as_str());
        }
        if let Ok(resp) = req.send().await {
            let len = resp.content_length().unwrap_or(0);
            if len > 0 {
                return len;
            }
        }
    }

    // ── 2. GET Range: bytes=0-0 ──────────────────────────────────────────────
    {
        let mut req = client
            .get(url)
            .header("User-Agent", crate::constants::USER_AGENT)
            .header("Range", "bytes=0-0");
        if let Some(ref c) = cookies {
            req = req.header("Cookie", c.as_str());
        }
        if let Ok(resp) = req.send().await {
            // Parse Content-Range: bytes 0-0/12345678  or  bytes */12345678
            if let Some(cr) = resp.headers().get("content-range") {
                if let Ok(s) = cr.to_str() {
                    if let Some(total_str) = s.split('/').last() {
                        if let Ok(total) = total_str.trim().parse::<u64>() {
                            if total > 0 {
                                return total;
                            }
                        }
                    }
                }
            }
            // Also check Content-Length on the range response itself
            let len = resp.content_length().unwrap_or(0);
            if len > 0 {
                return len;
            }
        }
    }

    // ── 3. Full GET with no Accept-Encoding — same client as streaming_download ─
    // create_client() sends Accept-Encoding: gzip which causes servers to use
    // chunked transfer encoding (no Content-Length). Use a bare client with only
    // User-Agent so the server sends uncompressed content with a real Content-Length,
    // exactly like streaming_download does. Drop the body immediately after headers.
    {
        if let Ok(bare_client) = reqwest::Client::builder()
            .user_agent(crate::constants::USER_AGENT)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
        {
            let mut req = bare_client.get(url);
            if let Some(c) = cookies {
                req = req.header("Cookie", c);
            }
            if let Ok(resp) = req.send().await {
                let len = resp.content_length().unwrap_or(0);
                if len > 0 {
                    return len;
                }
            }
        }
    }

    0
}

/// Return the number of free bytes available on the drive that contains `path`.
/// Walks up to the first existing ancestor so it works for not-yet-created paths.
pub(crate) fn available_bytes_at(path: &std::path::Path) -> u64 {
    // Find the highest existing ancestor
    let mut query = path.to_path_buf();
    loop {
        if query.exists() {
            break;
        }
        match query.parent() {
            Some(p) => query = p.to_path_buf(),
            None => break,
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        extern "system" {
            fn GetDiskFreeSpaceExW(
                lpDirectoryName: *const u16,
                lpFreeBytesAvailableToCaller: *mut u64,
                lpTotalNumberOfBytes: *mut u64,
                lpTotalNumberOfFreeBytes: *mut u64,
            ) -> i32;
        }
        let wide: Vec<u16> = query
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut free: u64 = 0;
        let mut total: u64 = 0;
        let mut total_free: u64 = 0;
        unsafe {
            if GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut total_free) != 0 {
                return free;
            }
        }
        0
    }

    #[cfg(not(windows))]
    {
        // Not implemented for non-Windows (this app targets Windows)
        let _ = query;
        0
    }
}

/// Return install path and available disk space. Download size is probed
/// separately by probe_download_size so the modal can show path/space instantly.
#[tauri::command]
pub async fn get_install_preflight(
    _url: String,
    _source_id: String,
    game_title: String,
    download_dir: Option<String>,
    install_dir: Option<String>,
    app_handle: AppHandle,
) -> Result<InstallPreflight, String> {
    use crate::engine::library_tracker::{get_game_folder, generate_game_slug};

    let game_slug = generate_game_slug(&game_title);

    // Install path: custom dir + slug/game, or default library path
    let install_path_buf = if let Some(ref dir) = install_dir {
        std::path::PathBuf::from(dir).join(&game_slug).join("game")
    } else {
        get_game_folder(&app_handle, &game_slug).join("game")
    };
    let install_path = install_path_buf.to_string_lossy().to_string();

    let download_folder = if let Some(ref dir) = download_dir {
        std::path::PathBuf::from(dir)
    } else {
        get_download_folder(&app_handle)
    };
    let download_path = download_folder.to_string_lossy().to_string();
    let available_bytes = available_bytes_at(&download_folder);

    Ok(InstallPreflight {
        download_size_bytes: 0,
        download_path,
        install_path,
        available_bytes,
    })
}

/// Probe the real archive size by mirroring the exact download pipeline:
/// source-level link resolution → host detection → webview URL capture (if needed)
/// → bare GET for Content-Length. Returns 0 if size cannot be determined.
#[tauri::command]
pub async fn probe_download_size(url: String, source_id: String, app_handle: AppHandle) -> ProbeResult {
    let zero = |resolved_url: String| ProbeResult { size: 0, resolved_url, cookies: None };

    let config = match SourceLoader::load_by_id(&source_id) {
        Ok(c) => c,
        Err(_) => return zero(url),
    };
    let cookies = UserSettings::get_cookies(&source_id);

    // Step 1: Source-level link resolution (same as run_smart_download_background)
    let resolved_url = if let Some(link_config) = &config.link_resolution {
        match resolve_link_on_demand(&url, link_config, cookies.as_deref()).await {
            Ok(resolved) => resolved,
            Err(_) => url.clone(),
        }
    } else {
        url.clone()
    };
    // Normalize protocol-relative URLs (//host.com/…) → https:// (mirrors run_smart_download_background)
    let resolved_url = if resolved_url.starts_with("//") {
        format!("https:{}", resolved_url)
    } else {
        resolved_url
    };

    // Step 2: Detect host (same as run_smart_download_background)
    let (_detected, host_config) = host_detector::detect_host_with_config(&resolved_url, config.hosts.as_ref());

    // Step 3: If webview host — open browser, capture CDN URL, read Content-Length
    if let Some(hc) = &host_config {
        if hc.download_method == DownloadMethod::Webview {
            log::info!("[probe_download_size] Webview host, opening browser to capture CDN URL");
            if let Some(webview_config) = &hc.webview_config {
                let downloader = WebViewDownloader::new(app_handle.clone());
                let result = downloader.get_download_url(&resolved_url, webview_config, None, true).await;
                log::info!("[probe_download_size] Webview probe: size={:?} cdn={:?}", result.file_size, result.download_url);
                return ProbeResult {
                    size: result.file_size.unwrap_or(0),
                    resolved_url: result.download_url.unwrap_or(resolved_url),
                    cookies: result.cookies,
                };
            }
            return zero(resolved_url);
        }
    }

    // Step 4: Non-webview host — run host-level resolver then bare GET
    let probe_client = reqwest::Client::builder()
        .user_agent(crate::constants::USER_AGENT)
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .unwrap_or_default();
    let http_client = HttpClient::new(probe_client);
    let manager = DownloadManager::new(http_client);
    let direct_url = manager.resolve_for_probe(&resolved_url, config.hosts.as_ref()).await;
    log::debug!("[probe_download_size] direct URL: {}", direct_url);

    // Bare GET — no Accept-Encoding so server must provide Content-Length
    if let Ok(client) = reqwest::Client::builder()
        .user_agent(crate::constants::USER_AGENT)
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
    {
        let mut req = client.get(&direct_url);
        if let Some(c) = cookies {
            req = req.header("Cookie", c);
        }
        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                let len = resp.content_length().unwrap_or(0);
                if len > 0 {
                    log::debug!("[probe_download_size] content-length: {}", len);
                    return ProbeResult { size: len, resolved_url: direct_url, cookies: None };
                }
            }
        }
    }

    zero(direct_url)
}
