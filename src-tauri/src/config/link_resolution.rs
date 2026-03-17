use serde::Deserialize;
use std::collections::HashMap;
use serde_json::Value as JsonValue;

use super::transformations::Transformation;
use super::extraction::MetadataExtractor;

#[derive(Debug, Clone, Deserialize)]
pub struct LinkResolutionConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    pub resolvers: HashMap<String, LinkResolver>,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Deserialize)]
pub struct LinkResolver {
    pub source_field: String,
    pub steps: Vec<ResolutionStep>,

    #[serde(default, rename = "metadata")]
    pub _metadata: Option<HashMap<String, MetadataExtractor>>,

    #[serde(default, rename = "timeout_ms")]
    pub _timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ResolutionStep {
    Fetch {
        #[serde(default = "default_true")]
        follow_redirects: bool,

        #[serde(default)]
        headers: Option<HashMap<String, String>>,

        #[serde(default)]
        timeout_ms: Option<u64>,
    },

    Extract {
        method: ExtractionMethodType,

        #[serde(default)]
        pattern: Option<String>,

        #[serde(default)]
        group: Option<usize>,

        #[serde(default)]
        selector: Option<String>,

        #[serde(default)]
        attribute: Option<String>,

        #[serde(default)]
        fallback: Option<String>,
    },

    Transform {
        transformations: Vec<Transformation>,
    },

    Wait {
        duration_ms: u64,
    },

    Custom {
        #[serde(rename = "name")]
        _name: String,
        #[serde(flatten)]
        _params: HashMap<String, JsonValue>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionMethodType {
    Regex,
    Selector,
    JsonPath,
    XPath,
    Text,
}
