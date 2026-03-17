/// Generate JavaScript for detecting successful login
pub fn get_login_detection_script(source_id: &str, session_cookie: &str, base_host: &str) -> String {
    format!(
        r#"
        (function() {{
            if (window.__AUTH_MONITOR_ACTIVE) return;
            window.__AUTH_MONITOR_ACTIVE = true;

            const SOURCE_ID = '{}';
            const SESSION_COOKIE = '{}';
            const BASE_HOST = '{}';

            function tryNotifySuccess() {{
                if (window.__TAURI__ && window.__TAURI__.event) {{
                    window.__TAURI__.event.emit('webview-login-success', {{
                        source_id: SOURCE_ID,
                        cookies: document.cookie
                    }}).catch(() => {{}});
                }}
            }}

            function checkAuth() {{
                const currentHost = window.location.hostname.toLowerCase();
                const baseHostLower = BASE_HOST.toLowerCase();
                const isOnBaseHost = currentHost === baseHostLower ||
                                     currentHost === 'www.' + baseHostLower ||
                                     currentHost.endsWith('.' + baseHostLower);

                const hasAuth = SESSION_COOKIE
                    ? document.cookie.includes(SESSION_COOKIE + '=')
                    : document.cookie.length > 0;

                if (isOnBaseHost && hasAuth) {{
                    tryNotifySuccess();
                    return true;
                }}
                return false;
            }}

            if (document.readyState === 'complete') {{
                checkAuth();
            }} else {{
                window.addEventListener('load', checkAuth);
            }}

            let checks = 0;
            const interval = setInterval(() => {{
                checks++;
                if (checkAuth() || checks > 30) {{
                    clearInterval(interval);
                }}
            }}, 1000);
        }})();
    "#,
        source_id, session_cookie, base_host
    )
}

/// Generate JavaScript for fetching with authentication
pub fn get_fetch_script(request_id: &str, url: &str) -> String {
    format!(
        r#"
        (async function() {{
            const REQUEST_ID = '{}';
            try {{
                const response = await fetch('{}', {{
                    method: 'GET',
                    credentials: 'include',
                    headers: {{
                        'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
                        'Accept-Language': navigator.language || 'en-US,en;q=0.9'
                    }}
                }});

                const body = await response.text();

                if (window.__TAURI__ && window.__TAURI__.event) {{
                    window.__TAURI__.event.emit('webview-fetch-result', {{
                        request_id: REQUEST_ID,
                        success: response.ok,
                        status: response.status,
                        body: body
                    }});
                }}
            }} catch (error) {{
                if (window.__TAURI__ && window.__TAURI__.event) {{
                    window.__TAURI__.event.emit('webview-fetch-result', {{
                        request_id: REQUEST_ID,
                        success: false,
                        status: 0,
                        body: '',
                        error: error.message
                    }});
                }}
            }}
        }})();
    "#,
        request_id, url
    )
}
