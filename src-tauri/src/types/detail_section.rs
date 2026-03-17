use serde::{Deserialize, Serialize};
use crate::config::paths::{PathStep, PathOrRef};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DetailSection {
    Hero {
        order: u32,
        config: HeroConfig,
    },
    Video {
        order: u32,
        title: String,
        icon: String,
        config: VideoConfig,
    },
    TextContent {
        order: u32,
        title: String,
        icon: String,
        config: TextContentConfig,
    },
    MetadataGrid {
        order: u32,
        title: String,
        icon: String,
        config: MetadataGridConfig,
    },
    NumberedSteps {
        order: u32,
        title: String,
        icon: String,
        config: NumberedStepsConfig,
    },
    AlertBox {
        order: u32,
        title: String,
        icon: String,
        style: String,
        config: AlertBoxConfig,
    },
    DownloadButtons {
        order: u32,
        title: String,
        icon: String,
        config: DownloadButtonsConfig,
    },
    /// Dynamic section type - fully config-driven, no code changes needed
    Dynamic {
        order: u32,
        title: String,
        icon: String,
        #[serde(default)]
        style: Option<String>,
        config: DynamicSectionConfig,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroConfig {
    pub background_image: SelectorConfig,
    pub title: SelectorConfig,
    pub subtitle: Option<SelectorConfig>,
    pub badges: Vec<BadgeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadgeConfig {
    pub label: String,
    pub icon: String,
    pub selector: String,
    pub contains: Option<String>,
    pub extract_after: Option<String>,
    pub extract_number: Option<bool>,
    pub suffix: Option<String>,
    pub style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorConfig {
    pub selector: String,
    pub attribute: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    pub selector: String,
    pub attribute: String,
    pub transform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContentConfig {
    pub selector: String,
    pub extract: String,
    pub max_length: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataGridConfig {
    pub items: Vec<MetadataItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataItem {
    pub label: String,
    pub icon: String,
    pub selector: String,
    pub contains: Option<String>,
    pub extract_after: Option<String>,
    pub attribute: Option<String>,
    pub render_as: Option<String>,
    pub style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumberedStepsConfig {
    pub selector: String,
    pub extract_method: String,
    pub start_after: Option<String>,
    pub end_before: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertBoxConfig {
    /// CSS selector for the container element (required unless `items` is provided)
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub parent_contains: Option<String>,
    /// CSS selector for individual items inside the container (required unless `items` is provided)
    #[serde(default)]
    pub items_selector: Option<String>,
    /// Static items defined directly in YAML — skips DOM scraping when present
    #[serde(default)]
    pub items: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadButtonsConfig {
    pub buttons: Vec<DownloadButtonConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadButtonConfig {
    pub label: String,
    pub selector: String,
    pub icon: String,
    pub style: String,

    #[serde(default = "default_button_action")]
    pub action: String,

    #[serde(default)]
    pub resolve_link: Option<bool>,

    /// If true, use the button's text content as the label instead of the config label
    #[serde(default)]
    pub use_text_as_label: Option<bool>,

    /// If set, only match elements whose text content contains this string
    #[serde(default)]
    pub contains_text: Option<String>,

    /// If true, automatically detect host and use smart download with fallback
    #[serde(default)]
    pub smart_download: Option<bool>,

    /// If false, button is locked (not yet supported by config author) — displayed but unclickable
    #[serde(default)]
    pub supported: Option<bool>,

    /// Optional warning message shown to the user before the download proceeds
    #[serde(default)]
    pub warning: Option<String>,

    // ===== NEW PATH-BASED RESOLUTION =====

    /// Named resolver to use (references `resolvers.{name}` in source config)
    #[serde(default)]
    pub resolver: Option<String>,

    /// Inline resolution path (alternative to resolver reference)
    #[serde(default)]
    pub resolution: Option<ButtonResolutionConfig>,

    /// Navigation path steps (simple inline path)
    #[serde(default)]
    pub path: Option<Vec<PathStep>>,
}

/// Configuration for button resolution with matching rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonResolutionConfig {
    /// Conditional path selection based on URL pattern
    #[serde(rename = "match")]
    #[serde(default)]
    pub match_rules: Option<Vec<ButtonMatchRule>>,

    /// Default resolver if no match
    #[serde(default)]
    pub default: Option<String>,

    /// Inline steps (alternative to match/default)
    #[serde(default)]
    pub steps: Option<Vec<PathStep>>,

    /// Fallback chain
    #[serde(default)]
    pub fallback: Option<Vec<FallbackEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonMatchRule {
    /// Condition to match (applied to extracted URL)
    pub when: MatchCondition,

    /// Resolver name to use
    #[serde(rename = "use")]
    pub resolver: Option<String>,

    /// Or inline path
    pub path: Option<PathOrRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchCondition {
    /// URL contains string
    Contains(String),
    /// URL matches regex
    Matches(String),
    /// URL starts with
    StartsWith(String),
    /// URL ends with
    EndsWith(String),
    /// Host equals
    HostEquals(String),
    /// Host contains
    HostContains(String),
    /// Always match (default case)
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackEntry {
    /// Try another resolver
    #[serde(rename = "try")]
    TryResolver(String),
    /// Return original URL
    ReturnOriginal,
    /// Return with error
    Error(String),
}

fn default_button_action() -> String {
    "open_link".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedSection {
    #[serde(rename = "type")]
    pub section_type: String,
    pub order: u32,
    pub title: Option<String>,
    pub icon: Option<String>,
    pub style: Option<String>,
    pub data: serde_json::Value,
}

// ===== DYNAMIC/GENERIC SECTION SUPPORT =====

/// Generic section config that can be defined entirely in YAML
/// This allows adding new section types without code changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicSectionConfig {
    /// The renderer to use (e.g., "key_value_grid", "list", "text_block", "button_group", "custom")
    pub renderer: String,

    /// Fields to extract - each field has a name and extraction config
    pub fields: std::collections::HashMap<String, FieldExtraction>,

    /// Optional styling hints
    #[serde(default)]
    pub style: Option<DynamicStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldExtraction {
    /// CSS selector to find the element
    pub selector: String,

    /// What to extract: "text", "html", "attribute"
    #[serde(default = "default_extract_type")]
    pub extract: String,

    /// Attribute name if extract is "attribute"
    #[serde(default)]
    pub attribute: Option<String>,

    /// Whether this field extracts multiple values
    #[serde(default)]
    pub multiple: bool,

    /// Optional regex pattern to apply after extraction
    #[serde(default)]
    pub pattern: Option<String>,

    /// Transformation to apply
    #[serde(default)]
    pub transform: Option<String>,

    /// Default value if extraction fails
    #[serde(default)]
    pub default: Option<String>,
}

fn default_extract_type() -> String {
    "text".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicStyle {
    #[serde(default)]
    pub variant: Option<String>,  // e.g., "info", "warning", "success", "error"
    #[serde(default)]
    pub columns: Option<u32>,     // For grid layouts
    #[serde(default)]
    pub compact: Option<bool>,
}