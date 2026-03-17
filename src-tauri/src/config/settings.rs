use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::auth::AuthConfig;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SettingSection {
    pub id: String,
    pub title: String,

    #[serde(default)]
    pub icon: Option<String>,

    #[serde(default)]
    pub description: Option<String>,

    pub components: Vec<SettingComponent>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SettingComponent {
    ButtonGroup {
        #[serde(default)]
        show_when: Option<VisibilityCondition>,
        #[serde(default)]
        hide_when: Option<VisibilityCondition>,
        buttons: Vec<ButtonConfig>,
    },

    Button {
        #[serde(default)]
        show_when: Option<VisibilityCondition>,
        #[serde(default)]
        hide_when: Option<VisibilityCondition>,
        #[serde(flatten)]
        config: ButtonConfig,
    },

    Toggle {
        id: String,
        label: String,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        default: Option<bool>,
        store_as: String,
        #[serde(default)]
        show_when: Option<VisibilityCondition>,
        #[serde(default)]
        hide_when: Option<VisibilityCondition>,
    },

    Input {
        id: String,
        label: String,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        placeholder: Option<String>,
        #[serde(default)]
        secret: bool,
        #[serde(default)]
        default: Option<String>,
        store_as: String,
        #[serde(default)]
        show_when: Option<VisibilityCondition>,
        #[serde(default)]
        hide_when: Option<VisibilityCondition>,
    },

    Select {
        id: String,
        label: String,
        #[serde(default)]
        description: Option<String>,
        options: Vec<SelectOption>,
        #[serde(default)]
        default: Option<String>,
        store_as: String,
        #[serde(default)]
        show_when: Option<VisibilityCondition>,
        #[serde(default)]
        hide_when: Option<VisibilityCondition>,
    },

    StatusCard {
        #[serde(default)]
        variant: Option<String>,
        #[serde(default)]
        icon: Option<String>,
        text: String,
        #[serde(default)]
        show_when: Option<VisibilityCondition>,
        #[serde(default)]
        hide_when: Option<VisibilityCondition>,
    },

    Text {
        content: String,
        #[serde(default)]
        variant: Option<String>,
        #[serde(default)]
        show_when: Option<VisibilityCondition>,
        #[serde(default)]
        hide_when: Option<VisibilityCondition>,
    },

    Divider {
        #[serde(default)]
        show_when: Option<VisibilityCondition>,
        #[serde(default)]
        hide_when: Option<VisibilityCondition>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ButtonConfig {
    pub label: String,

    #[serde(default)]
    pub icon: Option<String>,

    #[serde(default)]
    pub color: Option<String>,

    #[serde(default)]
    pub variant: Option<String>,

    pub action: ActionType,

    #[serde(default)]
    pub action_config: Option<ActionConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    OpenWebview,
    HttpRequest,
    StoreValue,
    ClearStorage,
    OpenExternal,
    Custom,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActionConfig {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub wait_for_cookie: Option<String>,
    #[serde(default)]
    pub close_on_domains: Option<Vec<String>>,
    #[serde(default)]
    pub store_cookies_as: Option<String>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    #[serde(default)]
    pub keys: Option<Vec<String>>,
    #[serde(default)]
    pub clear_webview: Option<bool>,
    #[serde(default)]
    pub link: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VisibilityCondition {
    pub key: String,
    #[serde(default)]
    pub exists: Option<bool>,
    #[serde(default)]
    pub equals: Option<serde_json::Value>,
    #[serde(default)]
    pub contains: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SettingDefinition {
    pub id: String,

    #[serde(rename = "type")]
    pub setting_type: SettingType,

    pub label: String,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub placeholder: Option<String>,

    #[serde(default)]
    pub default: Option<serde_json::Value>,

    #[serde(default)]
    pub required: bool,

    #[serde(default)]
    pub secret: bool,

    #[serde(default)]
    pub options: Option<Vec<SelectOption>>,

    #[serde(default)]
    pub config: Option<AuthConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingType {
    Text,
    Textarea,
    Toggle,
    Select,
    Number,
    Password,
    Auth,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}
