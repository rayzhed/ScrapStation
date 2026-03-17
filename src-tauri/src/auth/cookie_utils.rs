use tauri::webview::Cookie;

/// Convert cookies to a string format "name=value; name2=value2"
pub fn cookies_to_string(cookies: &[Cookie<'_>]) -> String {
    cookies
        .iter()
        .map(|c| format!("{}={}", c.name(), c.value()))
        .collect::<Vec<_>>()
        .join("; ")
}

/// Check if a cookie string contains the required session cookie
pub fn has_session_cookie(cookie_string: &str, session_cookie: &str) -> bool {
    if session_cookie.is_empty() {
        !cookie_string.is_empty()
    } else {
        cookie_string.contains(&format!("{}=", session_cookie))
    }
}
