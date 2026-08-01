use crate::auth::webview_auth;
use crate::engine::SourceLoader;
use crate::scraper::generic_detail_extractor;
use crate::settings::UserSettings;
use crate::types::detail_section::{DetailSection, ExtractedSection};
use crate::utils::create_client;
use tauri::AppHandle;

#[tauri::command]
pub async fn get_game_detail_sections(
    app: AppHandle,
    game_url: String,
    source_id: String,
) -> Result<Vec<ExtractedSection>, String> {
    let config = SourceLoader::load_by_id(&source_id)?;
    let cookies = UserSettings::get_cookies(&source_id);

    log::debug!(
        "[GameDetails] Loading details for source '{}', has cookies: {}",
        source_id,
        cookies.is_some()
    );

    let detail_page = config
        .extra
        .get("detail_page")
        .ok_or_else(|| format!("No detail_page config found for source '{}'", source_id))?;

    let sections_value = detail_page
        .get("sections")
        .ok_or("No sections found in detail_page config")?;

    let sections: Vec<DetailSection> = serde_json::from_value(sections_value.clone())
        .map_err(|e| format!("Failed to parse detail sections: {}", e))?;

    let cloudflare_protected = config
        .auth
        .as_ref()
        .map(|a| a.cloudflare_protected)
        .unwrap_or(false);
    let requires_webview = config
        .auth
        .as_ref()
        .map(|a| a.requires_webview_fetch)
        .unwrap_or(false);
    let has_webview_session = webview_auth::has_auth_webview(&source_id);
    let is_authenticated = cookies.is_some();

    let use_webview = (requires_webview && is_authenticated && has_webview_session)
        || has_webview_session;

    if cloudflare_protected {
        // CF-protected detail page: fetch HTML via TCP relay.
        let html = if use_webview {
            log::debug!("[GameDetails] CF session active — fetching detail page via TCP relay");
            webview_auth::fetch_via_webview(&app, &source_id, &game_url).await?
        } else {
            log::debug!("[GameDetails] CF source, no session — opening browser session directly");
            webview_auth::init_cloudflare_session(&app, &source_id, &config.base_url).await?;
            webview_auth::fetch_via_webview(&app, &source_id, &game_url).await?
        };
        return generic_detail_extractor::extract_sections_from_html(&html, &sections);
    }

    if use_webview {
        log::debug!("[GameDetails] Using WebView fetch for authenticated request");

        match webview_auth::fetch_authenticated(app.clone(), source_id.clone(), game_url.clone())
            .await
        {
            Ok(fetch_result) if fetch_result.success => {
                log::debug!(
                    "[GameDetails] WebView fetch successful, body length: {}",
                    fetch_result.body.len()
                );
                generic_detail_extractor::extract_sections_from_html(&fetch_result.body, &sections)
            }
            Ok(fetch_result) => {
                log::warn!(
                    "[GameDetails] WebView fetch failed: {:?}, falling back to HTTP",
                    fetch_result.error
                );
                generic_detail_extractor::extract_sections_with_cookies(
                    &game_url,
                    &sections,
                    cookies.as_deref(),
                )
                .await
            }
            Err(e) => {
                log::warn!(
                    "[GameDetails] WebView fetch error: {}, falling back to HTTP",
                    e
                );
                generic_detail_extractor::extract_sections_with_cookies(
                    &game_url,
                    &sections,
                    cookies.as_deref(),
                )
                .await
            }
        }
    } else {
        log::debug!("[GameDetails] Using HTTP fetch with cookies");
        generic_detail_extractor::extract_sections_with_cookies(
            &game_url,
            &sections,
            cookies.as_deref(),
        )
        .await
    }
}

/// Read a locally-cached image file and return its bytes.
/// Used by the frontend to display offline covers stored in app data.
#[tauri::command]
pub async fn read_local_image(path: String) -> Result<Vec<u8>, String> {
    tokio::fs::read(&path).await
        .map_err(|e| format!("Failed to read local image '{}': {}", path, e))
}

#[tauri::command]
pub async fn fetch_image(url: String) -> Result<Vec<u8>, String> {
    let client = create_client()?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch image: {}", e))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read image bytes: {}", e))?;

    Ok(bytes.to_vec())
}

/// Fetch an image through the CF-session WebView2 when direct loading is blocked.
/// Falls back to plain reqwest if no WebView session is active.
#[tauri::command]
pub async fn fetch_cf_image(app: AppHandle, source_id: String, url: String) -> Result<Vec<u8>, String> {
    if webview_auth::has_auth_webview(&source_id) {
        match webview_auth::fetch_binary_via_webview(&app, &source_id, &url).await {
            Ok(bytes) if !bytes.is_empty() => return Ok(bytes),
            Ok(_) => log::warn!("[FetchCfImage] Empty image response from WebView for '{}'", source_id),
            Err(e) => log::warn!("[FetchCfImage] WebView fetch failed for '{}': {}", source_id, e),
        }
    }

    // Fallback: plain reqwest (works when CF doesn't challenge, or for non-CF sources)
    let client = create_client()?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch image: {}", e))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read image bytes: {}", e))?;

    Ok(bytes.to_vec())
}

#[tauri::command]
pub async fn estimate_total_pages() -> Result<crate::config::PageEstimation, String> {
    Err("Total page estimation is not yet implemented".to_string())
}
