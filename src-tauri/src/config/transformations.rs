use serde::Deserialize;
use std::collections::HashMap;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transformation {
    Replace {
        pattern: String,
        replacement: String,
        #[serde(default)]
        regex: bool,
    },
    Trim,
    Lowercase,
    Uppercase,
    Capitalize,
    CollapseWhitespace,
    Truncate {
        max_length: usize,
        #[serde(default = "default_suffix")]
        suffix: String,
    },
    RemoveHtml,
    StripPrefix { prefix: String },
    StripSuffix { suffix: String },
    UrlNormalize {
        #[serde(default)]
        base_url: Option<String>,
        #[serde(default)]
        rules: Vec<UrlRule>,
    },
    Template { template: String },
    Extract {
        pattern: String,
        #[serde(default)]
        group: usize,
    },
    Split {
        delimiter: String,
        #[serde(default)]
        index: Option<usize>,
        #[serde(default)]
        join: Option<String>,
    },
    Append { text: String },
    Prepend { text: String },
    Default { value: String },
    Custom {
        #[serde(rename = "name")]
        _name: String,
        #[serde(flatten)]
        _params: HashMap<String, JsonValue>,
    },
}

fn default_suffix() -> String { "...".to_string() }

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UrlRule {
    PrependProtocol(String),
    PrependDomain(String),
    RemoveQueryParams,
    RemoveFragment,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    Contains {
        field: String,
        contains: String
    },
    Regex {
        field: String,
        pattern: String
    },
    Equals {
        field: String,
        equals: String
    },
    NotEmpty {
        field: String
    },
    And {
        conditions: Vec<Condition>,
    },
    Or {
        conditions: Vec<Condition>,
    },
}
