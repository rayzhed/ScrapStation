use serde::Deserialize;

use super::transformations::Condition;

#[derive(Debug, Clone, Deserialize)]
pub struct TagConfig {
    pub label: String,

    #[serde(default)]
    pub condition: Option<Condition>,

    #[serde(default)]
    pub color: Option<String>,

    #[serde(default)]
    pub background: Option<String>,

    #[serde(default)]
    pub icon: Option<String>,

    #[serde(default)]
    pub priority: i32,

    #[serde(default)]
    pub style: Option<TagStyle>,

    #[serde(default)]
    pub value_from: Option<ValueExtraction>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagStyle {
    Badge,
    Chip,
    Outline,
    Solid,
    Glow,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ValueExtraction {
    Static(String),
    FromField {
        field: String,
        #[serde(default)]
        prefix: Option<String>,
        #[serde(default)]
        suffix: Option<String>,
    },
    Template {
        template: String,
        #[serde(default)]
        fields: Vec<String>,
    },
}
