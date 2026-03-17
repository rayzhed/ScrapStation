use crate::constants::{DEFAULT_ICON, DEFAULT_COLOR};
use crate::config::paths::NavigationPath;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json::Value as JsonValue;

use super::transformations::{Transformation, Condition};
use super::extraction::MetadataExtractor;
use super::tags::TagConfig;
use super::settings::{SettingSection, SettingDefinition};
use super::link_resolution::LinkResolutionConfig;
use super::hosts::HostsConfig;

#[derive(Debug, Clone, Deserialize)]
pub struct SourceConfig {
    pub name: String,
    pub id: String,
    #[serde(rename = "type")]
    pub source_type: SourceType,
    pub base_url: String,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub ui: Option<UIMetadata>,

    #[serde(default)]
    pub urls: UrlConfig,

    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,

    #[serde(default)]
    pub selectors: Option<HashMap<String, PageSelectors>>,

    #[serde(default)]
    pub json_paths: Option<JsonPathsConfig>,

    #[serde(default)]
    pub attributes: Option<HashMap<String, Vec<String>>>,

    #[serde(default)]
    pub transformations: Option<HashMap<String, Vec<Transformation>>>,

    #[serde(default)]
    pub metadata_extraction: Option<HashMap<String, MetadataExtractor>>,

    #[serde(default)]
    pub custom_fields: Option<HashMap<String, CustomFieldExtractor>>,

    #[serde(default)]
    pub tags: Option<HashMap<String, TagConfig>>,

    #[serde(default)]
    pub default_values: Option<HashMap<String, String>>,

    #[serde(default)]
    pub field_mapping: Option<FieldMapping>,

    #[serde(default)]
    pub link_resolution: Option<LinkResolutionConfig>,

    #[serde(default)]
    pub setting_sections: Option<Vec<SettingSection>>,

    #[serde(default)]
    pub settings: Option<Vec<SettingDefinition>>,

    /// Host configurations for smart download (file hosters like GoFile, MEGA, etc.)
    #[serde(default)]
    pub hosts: Option<HostsConfig>,

    /// Authentication configuration
    #[serde(default)]
    pub auth: Option<AuthConfig>,

    /// Navigation paths for link resolution
    #[serde(default)]
    pub paths: Option<HashMap<String, NavigationPath>>,

    #[serde(flatten)]
    pub extra: HashMap<String, JsonValue>,
}

/// Authentication configuration for a source
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuthConfig {
    /// If true, all authenticated requests must go through WebView
    /// (needed for sites with Cloudflare or JavaScript-based protection)
    #[serde(default)]
    pub requires_webview_fetch: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    HtmlScraper,
    JsonApi,
    GraphqlApi,
    XmlApi,
    Custom,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UIMetadata {
    #[serde(default = "default_icon")]
    pub icon: String,

    #[serde(default = "default_color")]
    pub color: String,
}

fn default_icon() -> String {
    DEFAULT_ICON.to_string()
}

fn default_color() -> String {
    DEFAULT_COLOR.to_string()
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UrlConfig {
    pub main: Option<String>,
    pub page: Option<String>,
    pub search: Option<String>,
    /// Milliseconds to wait after the user stops typing before firing a search.
    /// Exposed to the frontend so each source can tune its own debounce.
    pub search_debounce_ms: Option<u64>,
    /// Minimum number of characters required before a search is fired.
    pub search_min_chars: Option<u32>,
    #[serde(rename = "detail")]
    pub _detail: Option<String>,
    #[serde(flatten)]
    pub _extra: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_window_seconds")]
    pub window_seconds: u64,

    #[serde(default = "default_min_delay_ms")]
    pub min_delay_ms: u64,

    #[serde(default = "default_burst_threshold")]
    pub burst_threshold: usize,

    #[serde(default, rename = "adaptive")]
    pub _adaptive: bool,
}

fn default_window_seconds() -> u64 { 10 }
fn default_min_delay_ms() -> u64 { 500 }
fn default_burst_threshold() -> usize { 5 }

#[derive(Debug, Clone, Deserialize)]
pub struct PageSelectors {
    pub container: String,
    pub fields: HashMap<String, FieldSelector>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FieldSelector {
    Simple(String),
    WithAttribute {
        selector: String,
        #[serde(default)]
        attribute: Option<String>,
        #[serde(default)]
        multiple: bool,
    },
    Static {
        #[serde(rename = "static")]
        value: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonPathsConfig {
    pub items: String,
    pub fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct FieldMapping {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub game_url: Option<String>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub author: Option<String>,

    #[serde(flatten)]
    pub custom: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CustomFieldExtractor {
    pub source: FieldSource,
    #[serde(default)]
    pub transformations: Option<Vec<Transformation>>,
    #[serde(default)]
    pub condition: Option<Condition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FieldSource {
    Static(String),
    FromField {
        field: String
    },
    Computed {
        template: String,
        #[serde(default)]
        fields: Vec<String>,
    },
    Json {
        #[serde(flatten)]
        value: JsonValue,
    },
}
