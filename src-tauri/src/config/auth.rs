use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    #[serde(default)]
    pub sso: Vec<SsoProvider>,

    #[serde(default)]
    pub login: Option<AuthEndpoint>,

    #[serde(default)]
    pub register: Option<AuthEndpoint>,

    #[serde(default)]
    pub logout: Option<LogoutConfig>,

    #[serde(default)]
    pub session_cookie: Option<String>,

    #[serde(default)]
    pub close_on_domains: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SsoProvider {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    pub auth_url: String,
    #[serde(default)]
    pub success_cookie: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthEndpoint {
    pub endpoint: String,
    #[serde(default = "default_post")]
    pub method: String,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub ajax: bool,
    #[serde(default)]
    pub fields: Vec<AuthField>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub success_cookies: Vec<String>,
    #[serde(default)]
    pub success_json: Option<JsonSuccessCheck>,
}

fn default_post() -> String { "POST".to_string() }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthField {
    pub id: String,
    pub label: String,
    #[serde(rename = "type", default = "default_text")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub placeholder: Option<String>,
}

fn default_text() -> String { "text".to_string() }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonSuccessCheck {
    pub field: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogoutConfig {
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub clear_cookies: bool,
}
