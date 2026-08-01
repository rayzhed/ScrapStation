use tauri::{AppHandle, Manager, Emitter, Listener, WebviewWindowBuilder, WebviewUrl};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use url::Url;
use crate::constants::{AUTH_WINDOW_WIDTH, AUTH_WINDOW_HEIGHT};
use crate::engine::SourceLoader;
use crate::config::SettingDefinition;
use crate::settings::UserSettings;

use super::cookie_utils::{cookies_to_string, has_session_cookie};
use super::domain_utils::is_matching_domain;
use super::scripts::{get_login_detection_script, get_fetch_script};

static AUTH_WEBVIEWS: Lazy<Arc<Mutex<HashMap<String, bool>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub const WEBVIEW_AUTH_MARKER: &str = "WEBVIEW_AUTH_ACTIVE";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    pub success: bool,
    pub cookies: Option<String>,
    pub username: Option<String>,
    pub error: Option<String>,
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    pub success: bool,
    pub status: u16,
    pub body: String,
    pub error: Option<String>,
}

/// Configuration for creating an auth window
struct AuthWindowConfig {
    source_id: String,
    source_name: String,
    base_url: Url,
    auth_url: String,
    session_cookie: String,
    close_on_domains: Vec<String>,
    storage_key: String,
}

pub fn has_auth_webview(source_id: &str) -> bool {
    AUTH_WEBVIEWS.lock().unwrap().get(source_id).copied().unwrap_or(false)
}

/// Open a hidden WebView to `base_url` and wait for Cloudflare's JS challenge
/// to resolve. The window is kept alive so subsequent `fetch_via_webview` calls
/// can run `fetch()` from inside it using WebView2's real Chrome TLS fingerprint.
pub async fn init_cloudflare_session(
    app: &AppHandle,
    source_id: &str,
    base_url: &str,
) -> Result<(), String> {
    if is_already_authenticated(source_id) {
        return Ok(());
    }

    let base_url_parsed: url::Url = base_url
        .parse()
        .map_err(|e| format!("Invalid base URL: {}", e))?;

    let window_id = format!("auth-{}", source_id);

    if app.get_webview_window(&window_id).is_none() {
        WebviewWindowBuilder::new(app, &window_id, WebviewUrl::External(base_url_parsed.clone()))
            .title("Connecting...")
            .visible(false)
            .inner_size(800.0, 600.0)
            .build()
            .map_err(|e| format!("Failed to create CF session window: {}", e))?;
    }

    let window = app
        .get_webview_window(&window_id)
        .ok_or_else(|| format!("Failed to get CF session window for '{}'", source_id))?;

    // Poll for cf_clearance instead of a fixed sleep.
    // CF JS challenge typically completes in 1–2 s; we exit as soon as the cookie appears.
    let start = std::time::Instant::now();
    let deadline = start + std::time::Duration::from_secs(10);
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        if let Ok(cookies) = window.cookies_for_url(base_url_parsed.clone()) {
            let cookie_str = cookies_to_string(&cookies);
            if has_session_cookie(&cookie_str, "cf_clearance") {
                log::info!("[CFSession] cf_clearance obtained for '{}' in {:.1}s",
                    source_id,
                    start.elapsed().as_secs_f32()
                );
                break;
            }
        }

        if std::time::Instant::now() >= deadline {
            log::warn!("[CFSession] No cf_clearance after 10 s for '{}', proceeding anyway", source_id);
            break;
        }
    }

    mark_authenticated(source_id);
    log::info!("[CFSession] Session ready for '{}'", source_id);
    Ok(())
}

/// Bind a loopback TCP listener, inject the script returned by `make_script(port)`
/// into `window`, then wait up to 30 s for the browser to HTTP-POST its result back.
/// Returns the raw POST body bytes. Chrome exempts loopback from mixed-content rules.
async fn tcp_relay_eval(
    window: &tauri::WebviewWindow,
    make_script: impl FnOnce(u16) -> String + Send,
) -> Result<Vec<u8>, String> {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to bind local listener: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to get local port: {}", e))?
        .port();

    let script = make_script(port);
    window
        .eval(&script)
        .map_err(|e| format!("Failed to inject script: {}", e))?;

    let (stream, _) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        listener.accept(),
    )
    .await
    .map_err(|_| "WebView fetch timed out after 30 s".to_string())?
    .map_err(|e| format!("TCP accept error: {}", e))?;

    let (reader_half, mut writer_half) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(reader_half);

    let mut content_length: usize = 0;
    let mut line = String::new();
    loop {
        line.clear();
        reader
            .read_line(&mut line)
            .await
            .map_err(|e| format!("Header read error: {}", e))?;
        let trimmed = line.trim_end_matches(|c: char| c == '\r' || c == '\n');
        if trimmed.is_empty() {
            break;
        }
        if trimmed.to_lowercase().starts_with("content-length:") {
            if let Some(v) = trimmed.splitn(2, ':').nth(1) {
                content_length = v.trim().parse().unwrap_or(0);
            }
        }
    }

    let mut body_bytes = vec![0u8; content_length];
    if content_length > 0 {
        reader
            .read_exact(&mut body_bytes)
            .await
            .map_err(|e| format!("Body read error: {}", e))?;
    }

    let _ = writer_half
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
        .await;

    Ok(body_bytes)
}

/// Fetch `url` from inside an existing CF-session WebView using a local TCP relay.
/// JS POSTs the response JSON to a Rust-owned `127.0.0.1:0` listener.
pub async fn fetch_via_webview(
    app: &AppHandle,
    source_id: &str,
    url: &str,
) -> Result<String, String> {
    let window_id = format!("auth-{}", source_id);
    let window = app
        .get_webview_window(&window_id)
        .ok_or_else(|| format!("No CF session window for '{}' — call init_cloudflare_session first", source_id))?;

    let url_js = serde_json::to_string(url).unwrap_or_else(|_| format!("\"{}\"", url));

    let body_bytes = tcp_relay_eval(&window, move |port| {
        format!(
            r#"(async () => {{
    try {{
        const resp = await fetch({url_js}, {{credentials: 'include'}});
        const body = await resp.text();
        const payload = JSON.stringify({{ok: resp.ok, status: resp.status, body: body}});
        await fetch('http://127.0.0.1:{port}/', {{method: 'POST', mode: 'no-cors', body: payload}});
    }} catch (e) {{
        const payload = JSON.stringify({{ok: false, status: 0, body: '', error: String(e)}});
        await fetch('http://127.0.0.1:{port}/', {{method: 'POST', mode: 'no-cors', body: payload}});
    }}
}})();"#,
            url_js = url_js,
            port = port
        )
    })
    .await?;

    log::debug!("[CFRelay] Received {} bytes from WebView", body_bytes.len());

    if body_bytes.is_empty() {
        return Err("WebView returned empty response (script may not have executed)".to_string());
    }

    let body_str = String::from_utf8_lossy(&body_bytes);
    let parsed: serde_json::Value = serde_json::from_str(&body_str)
        .map_err(|e| format!("Failed to parse WebView payload: {} | raw: {}", e, &body_str[..body_str.len().min(200)]))?;

    if let Some(err) = parsed.get("error").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
        return Err(format!("WebView fetch error: {}", err));
    }

    let ok = parsed.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    if !ok {
        let status = parsed.get("status").and_then(|v| v.as_u64()).unwrap_or(0);
        return Err(format!("HTTP error: {}", status));
    }

    parsed
        .get("body")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "No body field in WebView response payload".to_string())
}

/// Same as `fetch_via_webview` but returns raw bytes. JS encodes via `FileReader.readAsDataURL`;
/// Rust base64-decodes the result.
pub async fn fetch_binary_via_webview(
    app: &AppHandle,
    source_id: &str,
    url: &str,
) -> Result<Vec<u8>, String> {
    let window_id = format!("auth-{}", source_id);
    let window = app
        .get_webview_window(&window_id)
        .ok_or_else(|| format!("No CF session window for '{}'", source_id))?;

    let url_js = serde_json::to_string(url).unwrap_or_else(|_| format!("\"{}\"", url));

    let body_bytes = tcp_relay_eval(&window, move |port| {
        format!(
            r#"(async () => {{
    try {{
        const resp = await fetch({url_js}, {{credentials: 'include'}});
        const blob = await resp.blob();
        const dataUrl = await new Promise((res, rej) => {{
            const r = new FileReader();
            r.onloadend = () => res(r.result);
            r.onerror = rej;
            r.readAsDataURL(blob);
        }});
        const payload = JSON.stringify({{ok: resp.ok, status: resp.status, body: dataUrl || ''}});
        await fetch('http://127.0.0.1:{port}/', {{method: 'POST', mode: 'no-cors', body: payload}});
    }} catch (e) {{
        const payload = JSON.stringify({{ok: false, status: 0, body: '', error: String(e)}});
        await fetch('http://127.0.0.1:{port}/', {{method: 'POST', mode: 'no-cors', body: payload}});
    }}
}})();"#,
            url_js = url_js,
            port = port
        )
    })
    .await?;

    if body_bytes.is_empty() {
        return Err("WebView returned empty image response".to_string());
    }

    let body_str = String::from_utf8_lossy(&body_bytes);
    let parsed: serde_json::Value = serde_json::from_str(&body_str)
        .map_err(|e| format!("Failed to parse image payload: {}", e))?;

    if let Some(err) = parsed.get("error").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
        return Err(format!("Image fetch error: {}", err));
    }

    let data_url = parsed
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // data URL format: "data:<mime>;base64,<data>"
    let base64_data = data_url
        .find(',')
        .map(|i| &data_url[i + 1..])
        .ok_or("Invalid data URL: no comma separator")?;

    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(base64_data)
        .map_err(|e| format!("Base64 decode error: {}", e))
}

fn mark_authenticated(source_id: &str) {
    AUTH_WEBVIEWS.lock().unwrap().insert(source_id.to_string(), true);
}

fn clear_auth_state(source_id: &str) {
    AUTH_WEBVIEWS.lock().unwrap().remove(source_id);
}

fn is_already_authenticated(source_id: &str) -> bool {
    AUTH_WEBVIEWS.lock().unwrap().get(source_id).copied().unwrap_or(false)
}

/// Extract and validate cookies from a webview window with retry logic
async fn extract_cookies_with_retry(
    window: &tauri::WebviewWindow,
    base_url: &Url,
    session_cookie: &str,
    max_attempts: u32,
) -> Option<String> {
    for attempt in 1..=max_attempts {
        let cookies = window.cookies_for_url(base_url.clone()).unwrap_or_default();
        let cookie_string = cookies_to_string(&cookies);

        if has_session_cookie(&cookie_string, session_cookie) {
            return Some(cookie_string);
        }

        if attempt < max_attempts {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
    None
}

/// Handle successful authentication
async fn handle_auth_success(
    app: &AppHandle,
    source_id: &str,
    cookie_string: &str,
    storage_key: &str,
    window: &tauri::WebviewWindow,
) {
    mark_authenticated(source_id);

    if storage_key == "cookies" {
        let _ = UserSettings::set_cookies(source_id, cookie_string);
    } else {
        let _ = UserSettings::set_setting(
            source_id,
            storage_key,
            serde_json::Value::String(cookie_string.to_string()),
        );
    }

    // Hide instead of close — keeps the WebView alive for fetch_authenticated()
    // so that DDoS-Guard / Cloudflare JS challenge cookies remain active.
    let _ = window.hide();

    let _ = app.emit(
        "auth-complete",
        AuthResult {
            success: true,
            cookies: Some(cookie_string.to_string()),
            username: None,
            error: None,
            source_id: source_id.to_string(),
        },
    );
}

/// Emit auth cancelled event
fn emit_auth_cancelled(app: &AppHandle, source_id: &str) {
    let _ = app.emit(
        "auth-complete",
        AuthResult {
            success: false,
            cookies: None,
            username: None,
            error: Some("Authentication cancelled".to_string()),
            source_id: source_id.to_string(),
        },
    );
}

#[tauri::command]
pub async fn extract_webview_cookies(
    app: AppHandle,
    source_id: String,
) -> Result<AuthResult, String> {
    let config = SourceLoader::load_by_id(&source_id)?;
    let window_id = format!("auth-{}", source_id);

    let window = app
        .get_webview_window(&window_id)
        .ok_or("No auth window found. Please open the login window first.")?;

    let base_url = Url::parse(&config.base_url)
        .map_err(|e| format!("Invalid base URL: {}", e))?;

    let cookies = window
        .cookies_for_url(base_url)
        .map_err(|e| format!("Failed to get cookies: {}", e))?;

    if cookies.is_empty() {
        return Ok(AuthResult {
            success: false,
            cookies: None,
            username: None,
            error: Some("No cookies found. Please make sure you're logged in.".to_string()),
            source_id,
        });
    }

    let cookie_string = cookies_to_string(&cookies);

    let auth_config = config
        .settings
        .as_ref()
        .and_then(|s: &Vec<SettingDefinition>| {
            s.iter()
                .find(|s| matches!(s.setting_type, crate::config::SettingType::Auth))
        })
        .and_then(|s| s.config.as_ref());

    let session_cookie = auth_config
        .and_then(|c| c.session_cookie.clone())
        .unwrap_or_default();

    if has_session_cookie(&cookie_string, &session_cookie) {
        UserSettings::set_cookies(&source_id, &cookie_string)?;
        mark_authenticated(&source_id);
        let _ = window.hide();

        Ok(AuthResult {
            success: true,
            cookies: Some(cookie_string),
            username: None,
            error: None,
            source_id,
        })
    } else {
        let error_msg = if session_cookie.is_empty() {
            "No session cookies found. Please complete the login.".to_string()
        } else {
            format!(
                "Session cookie '{}' not found. Please complete the login.",
                session_cookie
            )
        };
        Ok(AuthResult {
            success: false,
            cookies: Some(cookie_string),
            username: None,
            error: Some(error_msg),
            source_id,
        })
    }
}

/// Build and open an auth window with the given configuration
async fn create_auth_window(app: &AppHandle, config: AuthWindowConfig) -> Result<(), String> {
    let window_id = format!("auth-{}", config.source_id);
    let base_host = config
        .base_url
        .host_str()
        .ok_or("Base URL has no host")?
        .to_string();

    clear_auth_state(&config.source_id);

    if let Some(existing) = app.get_webview_window(&window_id) {
        let _ = existing.close();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let init_script = get_login_detection_script(&config.source_id, &config.session_cookie, &base_host);

    // Clones for navigation handler
    let source_id_nav = config.source_id.clone();
    let session_cookie_nav = config.session_cookie.clone();
    let close_domains_nav = config.close_on_domains.clone();
    let storage_key_nav = config.storage_key.clone();
    let app_nav = app.clone();

    let visited_external = Arc::new(Mutex::new(false));
    let visited_external_nav = visited_external.clone();

    // Clones for close handler
    let source_id_close = config.source_id.clone();
    let app_close = app.clone();

    let window = WebviewWindowBuilder::new(
        app,
        &window_id,
        WebviewUrl::External(
            config
                .auth_url
                .parse()
                .map_err(|e| format!("Invalid URL: {}", e))?,
        ),
    )
    .title(format!("Login - {}", config.source_name))
    .inner_size(AUTH_WINDOW_WIDTH, AUTH_WINDOW_HEIGHT)
    .center()
    .resizable(true)
    .initialization_script(&init_script)
    .on_navigation(move |url| {
        let current_host = url.host_str().unwrap_or("").to_lowercase();
        let is_close_domain = is_matching_domain(&current_host, &close_domains_nav);

        if !is_close_domain {
            *visited_external_nav.lock().unwrap() = true;
        }

        let has_visited_external = *visited_external_nav.lock().unwrap();

        if is_close_domain && has_visited_external {
            let source_id = source_id_nav.clone();
            let session_cookie = session_cookie_nav.clone();
            let storage_key = storage_key_nav.clone();
            let app = app_nav.clone();

            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

                if is_already_authenticated(&source_id) {
                    let window_id = format!("auth-{}", source_id);
                    if let Some(window) = app.get_webview_window(&window_id) {
                        let _ = window.hide();
                    }
                    return;
                }

                let window_id = format!("auth-{}", source_id);
                if let Some(window) = app.get_webview_window(&window_id) {
                    let Ok(cfg) = SourceLoader::load_by_id(&source_id) else { return };
                    let Ok(base_url) = Url::parse(&cfg.base_url) else { return };

                    if let Some(cookie_string) =
                        extract_cookies_with_retry(&window, &base_url, &session_cookie, 5).await
                    {
                        handle_auth_success(&app, &source_id, &cookie_string, &storage_key, &window)
                            .await;
                    }
                }
            });
        }

        true
    })
    .build()
    .map_err(|e| format!("Failed to create auth window: {}", e))?;

    window.on_window_event(move |event| {
        if matches!(
            event,
            tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
        ) {
            if !is_already_authenticated(&source_id_close) {
                emit_auth_cancelled(&app_close, &source_id_close);
            }
            clear_auth_state(&source_id_close);
        }
    });

    // Setup login success listener
    let source_id_listener = config.source_id.clone();
    let session_cookie_listener = config.session_cookie.clone();
    let storage_key_listener = config.storage_key.clone();
    let app_listener = app.clone();

    app.listen("webview-login-success", move |event| {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(event.payload()) {
            let event_source = data.get("source_id").and_then(|v| v.as_str()).unwrap_or("");

            if event_source == source_id_listener && !is_already_authenticated(&source_id_listener)
            {
                let source_id = source_id_listener.clone();
                let session_cookie = session_cookie_listener.clone();
                let storage_key = storage_key_listener.clone();
                let app = app_listener.clone();
                let window_id = format!("auth-{}", source_id);

                if let Some(window) = app.get_webview_window(&window_id) {
                    if let Ok(cfg) = SourceLoader::load_by_id(&source_id) {
                        if let Ok(base_url) = Url::parse(&cfg.base_url) {
                            if let Ok(cookies) = window.cookies_for_url(base_url) {
                                if !cookies.is_empty() {
                                    let cookie_string = cookies_to_string(&cookies);

                                    if has_session_cookie(&cookie_string, &session_cookie) {
                                        mark_authenticated(&source_id);

                                        if storage_key == "cookies" {
                                            let _ = UserSettings::set_cookies(&source_id, &cookie_string);
                                        } else {
                                            let _ = UserSettings::set_setting(
                                                &source_id,
                                                &storage_key,
                                                serde_json::Value::String(cookie_string.clone()),
                                            );
                                        }

                                        let _ = window.hide();
                                        let _ = app.emit(
                                            "auth-complete",
                                            AuthResult {
                                                success: true,
                                                cookies: Some(cookie_string),
                                                username: None,
                                                error: None,
                                                source_id: source_id.clone(),
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn open_auth_window(
    app: AppHandle,
    source_id: String,
    provider_id: Option<String>,
) -> Result<(), String> {
    let config = SourceLoader::load_by_id(&source_id)?;

    let auth_setting = config
        .settings
        .as_ref()
        .and_then(|s: &Vec<SettingDefinition>| {
            s.iter()
                .find(|s| matches!(s.setting_type, crate::config::SettingType::Auth))
        })
        .ok_or("No auth configuration found for this source")?;

    let auth_config = auth_setting
        .config
        .as_ref()
        .ok_or("Auth setting has no configuration")?;

    let base_url = Url::parse(&config.base_url)
        .map_err(|e| format!("Invalid base URL: {}", e))?;

    let auth_url = if let Some(ref pid) = provider_id {
        let provider = auth_config
            .sso
            .iter()
            .find(|p| &p.id == pid)
            .ok_or_else(|| format!("SSO provider '{}' not found", pid))?;

        if provider.auth_url.starts_with("http") {
            provider.auth_url.clone()
        } else {
            format!("{}{}", config.base_url, provider.auth_url)
        }
    } else {
        config.base_url.clone()
    };

    create_auth_window(
        &app,
        AuthWindowConfig {
            source_id,
            source_name: config.name,
            base_url,
            auth_url,
            session_cookie: auth_config.session_cookie.clone().unwrap_or_default(),
            close_on_domains: auth_config.close_on_domains.clone(),
            storage_key: "cookies".to_string(),
        },
    )
    .await
}

pub async fn open_auth_window_generic(
    app: AppHandle,
    source_id: String,
    url: String,
    wait_for_cookie: Option<String>,
    close_on_domains: Vec<String>,
    store_cookies_as: Option<String>,
) -> Result<(), String> {
    let config = SourceLoader::load_by_id(&source_id)?;

    let base_url = Url::parse(&config.base_url)
        .map_err(|e| format!("Invalid base URL: {}", e))?;

    let full_url = if url.starts_with("http") {
        url
    } else {
        format!("{}{}", config.base_url, url)
    };

    create_auth_window(
        &app,
        AuthWindowConfig {
            source_id,
            source_name: config.name,
            base_url,
            auth_url: full_url,
            session_cookie: wait_for_cookie.unwrap_or_default(),
            close_on_domains,
            storage_key: store_cookies_as.unwrap_or_else(|| "cookies".to_string()),
        },
    )
    .await
}

#[tauri::command]
pub async fn fetch_authenticated(
    app: AppHandle,
    source_id: String,
    url: String,
) -> Result<FetchResult, String> {
    let window_id = format!("auth-{}", source_id);

    let window = app
        .get_webview_window(&window_id)
        .ok_or("Not logged in. Please authenticate first.")?;

    let request_id = format!("req-{}", fastrand::u64(..));
    let (tx, rx) = std::sync::mpsc::channel::<FetchResult>();
    let request_id_clone = request_id.clone();

    let listener_id = app.listen("webview-fetch-result", move |event| {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(event.payload()) {
            let event_req_id = data.get("request_id").and_then(|v| v.as_str()).unwrap_or("");

            if event_req_id == request_id_clone {
                let result = FetchResult {
                    success: data.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
                    status: data.get("status").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
                    body: data
                        .get("body")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    error: data.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
                };
                let _ = tx.send(result);
            }
        }
    });

    let fetch_script = get_fetch_script(&request_id, &url);
    window
        .eval(&fetch_script)
        .map_err(|e| format!("Failed to execute fetch: {}", e))?;

    let result = rx
        .recv_timeout(std::time::Duration::from_secs(30))
        .map_err(|_| "Request timeout".to_string())?;

    app.unlisten(listener_id);

    Ok(result)
}

#[tauri::command]
pub async fn get_auth_status(app: AppHandle, source_id: String) -> Result<AuthResult, String> {
    let window_id = format!("auth-{}", source_id);

    let has_webview = app.get_webview_window(&window_id).is_some();
    let is_registered = is_already_authenticated(&source_id);

    let stored = UserSettings::get_cookies(&source_id);

    let has_stored_cookies = stored
        .as_ref()
        .map(|s| !s.is_empty() && s.contains('='))
        .unwrap_or(false);

    let has_stored_auth = stored
        .as_ref()
        .map(|s| s == WEBVIEW_AUTH_MARKER)
        .unwrap_or(false);

    let is_logged_in = (has_webview && is_registered) || has_stored_auth || has_stored_cookies;

    Ok(AuthResult {
        success: is_logged_in,
        cookies: stored.clone(),
        username: None,
        error: if !is_logged_in {
            Some("Not logged in".to_string())
        } else {
            None
        },
        source_id,
    })
}

#[tauri::command]
pub async fn logout(app: AppHandle, source_id: String) -> Result<AuthResult, String> {
    let config = SourceLoader::load_by_id(&source_id)?;
    let window_id = format!("auth-{}", source_id);

    let window = if let Some(existing) = app.get_webview_window(&window_id) {
        existing
    } else {
        let base_url: url::Url = config
            .base_url
            .parse()
            .map_err(|e| format!("Invalid base URL: {}", e))?;

        WebviewWindowBuilder::new(&app, &window_id, WebviewUrl::External(base_url))
            .title("Logout")
            .inner_size(1.0, 1.0)
            .visible(false)
            .build()
            .map_err(|e| format!("Failed to create logout window: {}", e))?
    };

    window
        .clear_all_browsing_data()
        .map_err(|e| format!("Failed to clear browsing data: {}", e))?;

    let _ = window.close();

    clear_auth_state(&source_id);
    UserSettings::clear_cookies(&source_id)?;

    Ok(AuthResult {
        success: true,
        cookies: None,
        username: None,
        error: None,
        source_id,
    })
}

#[tauri::command]
pub async fn login_with_credentials(
    source_id: String,
    credentials: HashMap<String, String>,
) -> Result<AuthResult, String> {
    let config = SourceLoader::load_by_id(&source_id)?;

    let auth_setting = config
        .settings
        .as_ref()
        .and_then(|s: &Vec<SettingDefinition>| {
            s.iter()
                .find(|s| matches!(s.setting_type, crate::config::SettingType::Auth))
        })
        .ok_or("No auth configuration")?;

    let auth_config = auth_setting.config.as_ref().ok_or("No auth config")?;
    let login_config = auth_config.login.as_ref().ok_or("No login config")?;

    let url = if login_config.endpoint.starts_with("http") {
        login_config.endpoint.clone()
    } else {
        format!("{}{}", config.base_url, login_config.endpoint)
    };

    let body = login_config
        .body
        .as_ref()
        .map(|template| {
            let mut result = template.clone();
            for (key, value) in &credentials {
                result = result.replace(&format!("{{{}}}", key), value);
            }
            result
        })
        .unwrap_or_default();

    let jar = std::sync::Arc::new(reqwest::cookie::Jar::default());
    let client = reqwest::Client::builder()
        .cookie_provider(jar.clone())
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let mut request = client.post(&url);

    if let Some(ref ct) = login_config.content_type {
        request = request.header("Content-Type", ct);
    }
    if login_config.ajax {
        request = request.header("X-Requested-With", "XMLHttpRequest");
    }
    request = request.header("Referer", &config.base_url).body(body);

    let response = request
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let cookies: Vec<String> = response
        .cookies()
        .map(|c| format!("{}={}", c.name(), c.value()))
        .collect();

    let cookie_string = cookies.join("; ");

    let success = if !login_config.success_cookies.is_empty() {
        login_config
            .success_cookies
            .iter()
            .all(|req| cookies.iter().any(|c| c.starts_with(&format!("{}=", req))))
    } else {
        !cookies.is_empty()
    };

    if success {
        UserSettings::set_cookies(&source_id, &cookie_string)?;
    }

    Ok(AuthResult {
        success,
        cookies: if success { Some(cookie_string) } else { None },
        username: credentials.get("login_name").cloned(),
        error: if !success {
            Some("Login failed".to_string())
        } else {
            None
        },
        source_id,
    })
}

#[tauri::command]
pub async fn set_manual_cookies(source_id: String, cookies: String) -> Result<AuthResult, String> {
    UserSettings::set_cookies(&source_id, &cookies)?;

    Ok(AuthResult {
        success: true,
        cookies: Some(cookies),
        username: None,
        error: None,
        source_id,
    })
}
