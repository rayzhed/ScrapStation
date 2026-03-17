use crate::auth::webview_auth;
use crate::constants::USER_AGENT;
use crate::engine::SourceLoader;
use crate::settings::UserSettings;
use std::collections::HashMap;

#[derive(serde::Serialize)]
pub struct AuthResult {
    pub success: bool,
    pub message: String,
    pub username: Option<String>,
}

#[tauri::command]
pub async fn source_login(
    source_id: String,
    credentials: HashMap<String, String>,
) -> Result<AuthResult, String> {
    let config = SourceLoader::load_by_id(&source_id)?;
    let base_url = &config.base_url;

    let auth_setting = config
        .settings
        .as_ref()
        .and_then(|s| {
            s.iter()
                .find(|s| matches!(s.setting_type, crate::config::SettingType::Auth))
        })
        .ok_or("No auth configuration found for this source")?;

    let auth_config = auth_setting.config.as_ref().ok_or("Auth setting has no config")?;
    let login_config = auth_config.login.as_ref().ok_or("No login configuration found")?;

    let mut body = login_config.body.clone().unwrap_or_default();
    for (key, value) in &credentials {
        body = body.replace(&format!("{{{}}}", key), &urlencoding::encode(value));
    }

    let url = format!("{}{}", base_url, login_config.endpoint);

    let client = reqwest::Client::builder()
        .cookie_store(true)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut request = client
        .post(&url)
        .header(
            "Content-Type",
            login_config
                .content_type
                .as_deref()
                .unwrap_or("application/x-www-form-urlencoded"),
        )
        .header("Origin", base_url)
        .header("Referer", &format!("{}/", base_url))
        .body(body);

    if login_config.ajax {
        request = request.header("X-Requested-With", "XMLHttpRequest");
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Login request failed: {}", e))?;

    let cookies_to_save: Vec<String> = response
        .cookies()
        .map(|c| format!("{}={}", c.name(), c.value()))
        .collect();

    let success_cookies = &login_config.success_cookies;
    let has_success_cookies = success_cookies.is_empty()
        || success_cookies
            .iter()
            .any(|name| cookies_to_save.iter().any(|c| c.starts_with(&format!("{}=", name))));

    if has_success_cookies && !cookies_to_save.is_empty() {
        let cookie_string = cookies_to_save.join("; ");
        UserSettings::set_cookies(&source_id, &cookie_string)?;

        if let Some(username) = credentials.get("login_name").or(credentials.get("username")) {
            UserSettings::set_setting(&source_id, "auth_username", serde_json::json!(username))?;
        }

        Ok(AuthResult {
            success: true,
            message: "Login successful".to_string(),
            username: credentials
                .get("login_name")
                .or(credentials.get("username"))
                .cloned(),
        })
    } else {
        Ok(AuthResult {
            success: false,
            message: "Login failed - invalid credentials".to_string(),
            username: None,
        })
    }
}

#[tauri::command]
pub async fn source_register(
    source_id: String,
    credentials: HashMap<String, String>,
) -> Result<AuthResult, String> {
    let config = SourceLoader::load_by_id(&source_id)?;
    let base_url = &config.base_url;

    let auth_setting = config
        .settings
        .as_ref()
        .and_then(|s| {
            s.iter()
                .find(|s| matches!(s.setting_type, crate::config::SettingType::Auth))
        })
        .ok_or("No auth configuration found for this source")?;

    let auth_config = auth_setting.config.as_ref().ok_or("Auth setting has no config")?;
    let register_config = auth_config
        .register
        .as_ref()
        .ok_or("No register configuration found")?;

    let mut body = register_config.body.clone().unwrap_or_default();
    for (key, value) in &credentials {
        body = body.replace(&format!("{{{}}}", key), &urlencoding::encode(value));
    }

    let url = format!("{}{}", base_url, register_config.endpoint);

    let client = reqwest::Client::builder()
        .cookie_store(true)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut request = client
        .post(&url)
        .header(
            "Content-Type",
            register_config
                .content_type
                .as_deref()
                .unwrap_or("application/x-www-form-urlencoded"),
        )
        .header("Origin", base_url)
        .header("Referer", &format!("{}/", base_url))
        .body(body);

    if register_config.ajax {
        request = request.header("X-Requested-With", "XMLHttpRequest");
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Registration request failed: {}", e))?;
    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    if let Some(success_check) = &register_config.success_json {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
            let field_value = json
                .get(&success_check.field)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if field_value == success_check.value {
                return Ok(AuthResult {
                    success: true,
                    message: "Registration successful! You can now login.".to_string(),
                    username: credentials.get("reg_username").cloned(),
                });
            }
        }
    }

    if response_text.contains("error") || response_text.contains("Error") {
        Ok(AuthResult {
            success: false,
            message: "Registration failed - please check your details".to_string(),
            username: None,
        })
    } else {
        Ok(AuthResult {
            success: true,
            message: "Registration completed".to_string(),
            username: credentials.get("reg_username").cloned(),
        })
    }
}

#[tauri::command]
pub async fn source_logout(source_id: String) -> Result<(), String> {
    UserSettings::clear_cookies(&source_id)?;
    UserSettings::clear_setting(&source_id, "auth_username")
}

#[tauri::command]
pub async fn start_sso_login(source_id: String, provider_id: String) -> Result<String, String> {
    let config = SourceLoader::load_by_id(&source_id)?;

    let auth_setting = config
        .settings
        .as_ref()
        .and_then(|s| {
            s.iter()
                .find(|s| matches!(s.setting_type, crate::config::SettingType::Auth))
        })
        .ok_or("No auth configuration found for this source")?;

    let auth_config = auth_setting.config.as_ref().ok_or("Auth setting has no config")?;

    let provider = auth_config
        .sso
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("SSO provider '{}' not found", provider_id))?;

    Ok(format!("{}{}", config.base_url, provider.auth_url))
}

#[tauri::command]
pub async fn check_auth_status(source_id: String) -> Result<AuthResult, String> {
    let cookies = UserSettings::get_cookies(&source_id);
    let username = UserSettings::get_setting(&source_id, "auth_username")
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    let config = SourceLoader::load_by_id(&source_id)?;

    let auth_setting = config.settings.as_ref().and_then(|s| {
        s.iter()
            .find(|s| matches!(s.setting_type, crate::config::SettingType::Auth))
    });

    if let Some(auth_setting) = auth_setting {
        if let Some(auth_config) = &auth_setting.config {
            if let Some(session_cookie) = &auth_config.session_cookie {
                if let Some(cookies_str) = &cookies {
                    if cookies_str.contains(&format!("{}=", session_cookie)) {
                        return Ok(AuthResult {
                            success: true,
                            message: "Logged in".to_string(),
                            username,
                        });
                    }
                }
            }
        }
    }

    if cookies.is_some() && username.is_some() {
        Ok(AuthResult {
            success: true,
            message: "Logged in".to_string(),
            username,
        })
    } else {
        Ok(AuthResult {
            success: false,
            message: "Not logged in".to_string(),
            username: None,
        })
    }
}

#[tauri::command]
pub async fn set_source_cookies(source_id: String, cookies: String) -> Result<(), String> {
    UserSettings::set_cookies(&source_id, &cookies)
}

#[tauri::command]
pub async fn get_source_cookies(source_id: String) -> Result<Option<String>, String> {
    Ok(UserSettings::get_cookies(&source_id))
}

#[tauri::command]
pub async fn clear_source_cookies(source_id: String) -> Result<(), String> {
    UserSettings::clear_cookies(&source_id)
}

#[tauri::command]
pub async fn execute_action(
    app: tauri::AppHandle,
    source_id: String,
    action: String,
    config: serde_json::Value,
) -> Result<serde_json::Value, String> {
    match action.as_str() {
        "open_webview" => {
            let url = config
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'url' in action config")?;

            let wait_for_cookie = config
                .get("wait_for_cookie")
                .and_then(|v| v.as_str())
                .map(String::from);

            let close_on_domains: Vec<String> = config
                .get("close_on_domains")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let store_cookies_as = config
                .get("store_cookies_as")
                .and_then(|v| v.as_str())
                .map(String::from);

            webview_auth::open_auth_window_generic(
                app,
                source_id,
                url.to_string(),
                wait_for_cookie,
                close_on_domains,
                store_cookies_as,
            )
            .await?;

            Ok(serde_json::json!({ "status": "webview_opened" }))
        }

        "clear_storage" => {
            let keys: Vec<String> = config
                .get("keys")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let clear_webview = config
                .get("clear_webview")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            for key in &keys {
                UserSettings::clear_setting(&source_id, key)?;
            }

            if clear_webview {
                let _ = webview_auth::logout(app, source_id).await;
            }

            Ok(serde_json::json!({ "status": "cleared", "keys": keys }))
        }

        "store_value" => {
            let key = config
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'key' in action config")?;

            let value = config.get("value").ok_or("Missing 'value' in action config")?;

            UserSettings::set_setting(&source_id, key, value.clone())?;

            Ok(serde_json::json!({ "status": "stored", "key": key }))
        }

        "open_external" => {
            let link = config
                .get("link")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'link' in action config")?;

            super::download_commands::open_url_in_browser(link.to_string()).await?;

            Ok(serde_json::json!({ "status": "opened" }))
        }

        _ => Err(format!("Unknown action: {}", action)),
    }
}
