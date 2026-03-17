use crate::auth::webview_auth;
use crate::config::GameCard;
use crate::constants::{DEFAULT_ICON, DEFAULT_COLOR};
use crate::engine::{SourceLoader, UniversalScraper};
use crate::settings::UserSettings;
use crate::utils::{create_client, HttpClient};
use tauri::AppHandle;

use super::helpers::{apply_rate_limit, build_url, convert_to_game_cards};

/// Return the path to the user's sources directory (where YAML configs live).
#[tauri::command]
pub async fn get_sources_folder_path() -> Result<String, String> {
    let dir = SourceLoader::sources_dir();
    // Ensure the directory exists before returning the path
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create sources directory: {}", e))?;
    }
    Ok(dir.to_string_lossy().to_string())
}

/// Open the sources directory in the system file explorer.
#[tauri::command]
pub async fn open_sources_folder() -> Result<(), String> {
    let dir = SourceLoader::sources_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create sources directory: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&dir)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&dir)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    Ok(())
}

/// Install a source config YAML file into the sources directory from a filesystem path.
/// Called by the frontend after the user drops a YAML file onto the app window
/// (path comes from Tauri's onDragDropEvent, which captures OS-level file drops).
#[tauri::command]
pub async fn install_source_from_path(path: String) -> Result<String, String> {
    let src = std::path::PathBuf::from(&path);

    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext != "yaml" && ext != "yml" {
        return Err(format!("'{}' is not a .yaml or .yml file", src.file_name().unwrap_or_default().to_string_lossy()));
    }

    let filename = src.file_name()
        .ok_or("Invalid path")?
        .to_string_lossy()
        .to_string();

    let content = std::fs::read_to_string(&src)
        .map_err(|e| format!("Failed to read '{}': {}", filename, e))?;

    let dir = SourceLoader::sources_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create sources directory: {}", e))?;
    }

    std::fs::write(dir.join(&filename), content.as_bytes())
        .map_err(|e| format!("Failed to write config: {}", e))?;

    log::info!("[SourceInstall] Installed '{}' from {:?}", filename, src);
    Ok(filename)
}

/// Install a source config YAML file from raw content (fallback for non-OS drops).
#[tauri::command]
pub async fn install_source_config(filename: String, content: String) -> Result<(), String> {
    if !filename.ends_with(".yaml") && !filename.ends_with(".yml") {
        return Err("Only .yaml or .yml files are accepted".to_string());
    }

    let safe_name = std::path::Path::new(&filename)
        .file_name()
        .ok_or("Invalid filename")?
        .to_string_lossy()
        .to_string();

    let dir = SourceLoader::sources_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create sources directory: {}", e))?;
    }

    std::fs::write(dir.join(&safe_name), content.as_bytes())
        .map_err(|e| format!("Failed to write config: {}", e))?;

    log::info!("[SourceInstall] Installed '{}'", safe_name);
    Ok(())
}

#[tauri::command]
pub async fn list_sources() -> Result<Vec<String>, String> {
    let configs = SourceLoader::load_all()?;
    Ok(configs.into_iter().map(|c| c.id).collect())
}

/// Fetch a URL with optional cookie header, returning the response body.
/// Returns a user-friendly error if a bot-protection challenge is detected.
async fn fetch_page(url: &str, cookies: Option<&str>) -> Result<String, String> {
    let client = create_client()?;
    let mut request = client.get(url);

    if let Some(cookie_str) = cookies {
        request = request.header("Cookie", cookie_str);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let content = response
        .text()
        .await
        .map_err(|e| format!("Failed to get response: {}", e))?;

    // Detect Cloudflare JS challenge (returns 200 but with no real content)
    if content.contains("challenge-running")
        || content.contains("cf-browser-verification")
        || (content.contains("Just a moment") && content.contains("cloudflare"))
    {
        return Err(
            "Cloudflare protection detected. Please log in via the source settings to establish a browser session first.".to_string()
        );
    }

    // Detect DDoS-Guard challenge
    if content.contains("ddos-guard.net")
        || content.contains("__ddg1_")
        || content.contains("DDoS-Guard")
    {
        return Err(
            "DDoS-Guard protection detected. Please log in via the source settings to establish a browser session first.".to_string()
        );
    }

    Ok(content)
}

/// Fetch page content using WebView if available, falling back to plain HTTP.
async fn fetch_page_with_webview(
    app: AppHandle,
    source_id: &str,
    url: &str,
    cookies: Option<&str>,
    has_webview_session: bool,
) -> Result<String, String> {
    if has_webview_session {
        match webview_auth::fetch_authenticated(app, source_id.to_string(), url.to_string()).await {
            Ok(result) if result.success => return Ok(result.body),
            Ok(result) => {
                log::warn!(
                    "[SourceLoad] WebView fetch failed: {:?}, falling back to HTTP",
                    result.error
                );
            }
            Err(e) => {
                log::warn!(
                    "[SourceLoad] WebView fetch error: {}, falling back to HTTP",
                    e
                );
            }
        }
    }

    fetch_page(url, cookies).await
}

#[tauri::command]
pub async fn load_dynamic_source(
    app: AppHandle,
    source_id: String,
    page: Option<u32>,
) -> Result<Vec<GameCard>, String> {
    let config = SourceLoader::load_by_id(&source_id)?;
    apply_rate_limit(&source_id, &config).await;

    let page = page.unwrap_or(1);
    let url = build_url(&config, page)?;

    let requires_webview = config
        .auth
        .as_ref()
        .map(|a| a.requires_webview_fetch)
        .unwrap_or(false);
    let cookies = UserSettings::get_cookies(&source_id);
    let is_authenticated = cookies.is_some();
    let has_webview_session = webview_auth::has_auth_webview(&source_id);

    log::debug!(
        "[SourceLoad] source='{}' requires_webview={} is_authenticated={} has_webview_session={}",
        source_id,
        requires_webview,
        is_authenticated,
        has_webview_session
    );

    // Use WebView when available (bypasses bot protection opportunistically),
    // or when the source explicitly requires it.
    // Only hard-error when the source *requires* WebView but no session exists.
    let use_webview = has_webview_session;

    let content = if requires_webview && !use_webview {
        return Err(
            "This source requires an active browser session. Please log in via the source settings first.".to_string()
        );
    } else if use_webview {
        fetch_page_with_webview(app, &source_id, &url, cookies.as_deref(), has_webview_session)
            .await?
    } else {
        fetch_page(&url, cookies.as_deref()).await?
    };

    log::debug!("[SourceLoad] Fetched {} bytes for '{}'", content.len(), source_id);

    let client = create_client()?;
    let http_client = HttpClient::new(client);
    let raw_items =
        UniversalScraper::scrape_with_client(&config, &content, false, Some(&http_client)).await?;

    log::info!("[SourceLoad] '{}' page {} → {} games", source_id, page, raw_items.len());
    convert_to_game_cards(raw_items, &config)
}

#[tauri::command]
pub async fn search_dynamic_source(
    app: AppHandle,
    source_id: String,
    query: String,
) -> Result<Vec<GameCard>, String> {
    if query.trim().is_empty() {
        return Err("Search query cannot be empty".to_string());
    }

    let config = SourceLoader::load_by_id(&source_id)?;
    apply_rate_limit(&source_id, &config).await;

    let search_template = config
        .urls
        .search
        .as_ref()
        .ok_or("No search URL template in config")?;

    let url = format!(
        "{}{}",
        config.base_url,
        search_template.replace("{query}", &urlencoding::encode(&query))
    );

    let requires_webview = config
        .auth
        .as_ref()
        .map(|a| a.requires_webview_fetch)
        .unwrap_or(false);
    let cookies = UserSettings::get_cookies(&source_id);
    let is_authenticated = cookies.is_some();
    let has_webview_session = webview_auth::has_auth_webview(&source_id);

    // Source requires WebView auth but user hasn't logged in at all
    if requires_webview && !is_authenticated && !has_webview_session {
        return Err(
            "This source requires authentication. Please log in via the source settings first."
                .to_string(),
        );
    }

    let use_webview = (requires_webview && is_authenticated && has_webview_session)
        || has_webview_session;

    let content = if use_webview {
        fetch_page_with_webview(app, &source_id, &url, cookies.as_deref(), has_webview_session)
            .await?
    } else {
        fetch_page(&url, cookies.as_deref()).await?
    };

    let client = create_client()?;
    let http_client = HttpClient::new(client);
    let raw_items =
        UniversalScraper::scrape_with_client(&config, &content, true, Some(&http_client)).await?;

    convert_to_game_cards(raw_items, &config)
}

#[tauri::command]
pub async fn get_source_metadata(source_id: String) -> Result<serde_json::Value, String> {
    let config = SourceLoader::load_by_id(&source_id)?;

    let (icon, color) = if let Some(ui) = &config.ui {
        (ui.icon.clone(), ui.color.clone())
    } else {
        (DEFAULT_ICON.to_string(), DEFAULT_COLOR.to_string())
    };

    Ok(serde_json::json!({
        "id": config.id,
        "name": config.name,
        "description": config.description,
        "base_url": config.base_url,
        "icon": icon,
        "color": color,
        "search_debounce_ms": config.urls.search_debounce_ms.unwrap_or(300),
        "search_min_chars": config.urls.search_min_chars.unwrap_or(1),
        "notices": config.extra.get("notices").cloned().unwrap_or(serde_json::Value::Array(vec![])),
    }))
}
