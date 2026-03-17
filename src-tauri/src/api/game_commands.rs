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

    // Check if we should use WebView for authenticated requests
    // This is needed for sites with Cloudflare or JavaScript-based protections
    let requires_webview = config
        .auth
        .as_ref()
        .map(|a| a.requires_webview_fetch)
        .unwrap_or(false);

    let has_webview_session = webview_auth::has_auth_webview(&source_id);
    let is_authenticated = cookies.is_some();

    // Use WebView fetch if:
    // 1. Source requires it AND user is authenticated AND WebView session is active, OR
    // 2. WebView session is currently active (user just logged in)
    let use_webview = (requires_webview && is_authenticated && has_webview_session)
        || has_webview_session;

    if use_webview {
        log::debug!("[GameDetails] Using WebView fetch for authenticated request");

        match webview_auth::fetch_authenticated(app.clone(), source_id.clone(), game_url.clone())
            .await
        {
            Ok(fetch_result) => {
                if !fetch_result.success {
                    log::warn!(
                        "[GameDetails] WebView fetch failed: {:?}, falling back to HTTP",
                        fetch_result.error
                    );
                    // Fall back to HTTP request
                    generic_detail_extractor::extract_sections_with_cookies(
                        &game_url,
                        &sections,
                        cookies.as_deref(),
                    )
                    .await
                } else {
                    log::debug!(
                        "[GameDetails] WebView fetch successful, body length: {}",
                        fetch_result.body.len()
                    );
                    generic_detail_extractor::extract_sections_from_html(
                        &fetch_result.body,
                        &sections,
                    )
                }
            }
            Err(e) => {
                log::warn!(
                    "[GameDetails] WebView fetch error: {}, falling back to HTTP",
                    e
                );
                // Fall back to HTTP request
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

#[tauri::command]
pub async fn estimate_total_pages() -> Result<crate::config::PageEstimation, String> {
    Err("Total page estimation is not yet implemented".to_string())
}
