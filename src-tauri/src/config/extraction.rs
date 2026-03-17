use serde::Deserialize;
use std::collections::HashMap;
use serde_json::Value as JsonValue;

use super::transformations::{Transformation, Condition};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetadataExtractor {
    Boolean {
        #[serde(default)]
        condition: Option<Condition>,
    },
    String {
        #[serde(default)]
        extraction: Option<ExtractionMethod>,
        #[serde(default)]
        transformations: Option<Vec<Transformation>>,
        #[serde(default)]
        fallback: Option<String>,
    },
    Date {
        #[serde(default)]
        extraction: Option<ExtractionMethod>,
        #[serde(default)]
        input_format: Option<String>,
        #[serde(default)]
        output_format: Option<String>,
        #[serde(default)]
        fallback: Option<String>,
    },
    List {
        #[serde(default)]
        extraction: Option<ListExtractionMethod>,
        #[serde(default)]
        join_with: Option<String>,
    },
    Number {
        #[serde(default)]
        extraction: Option<ExtractionMethod>,
        #[serde(default)]
        fallback: Option<f64>,
    },
    Custom {
        #[serde(flatten)]
        _config: HashMap<String, JsonValue>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum ExtractionMethod {
    Pattern {
        pattern: String,
        #[serde(default)]
        in_field: Option<String>,
    },
    Regex {
        pattern: String,
        #[serde(default)]
        in_field: Option<String>,
        #[serde(default)]
        group: usize,
    },
    NextWord {
        after_pattern: String,
        #[serde(default)]
        in_field: Option<String>,
    },
    Between {
        start: String,
        end: String,
        #[serde(default)]
        in_field: Option<String>,
    },
    FromField {
        field: String,
    },
    JsonPath {
        path: String,
    },
    Custom {
        #[serde(rename = "name")]
        _name: String,
        #[serde(flatten)]
        _params: HashMap<String, JsonValue>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum ListExtractionMethod {
    Keywords {
        #[serde(default)]
        in_field: Option<String>,
        keywords: Vec<KeywordMatch>,
    },
    Split {
        in_field: String,
        delimiter: String,
    },
    Regex {
        #[serde(default)]
        in_field: Option<String>,
        pattern: String,
    },
    Custom {
        #[serde(rename = "name")]
        _name: String,
        #[serde(flatten)]
        _params: HashMap<String, JsonValue>,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct KeywordMatch {
    pub pattern: String,
    pub value: String,
    #[serde(default)]
    pub regex: bool,
}
