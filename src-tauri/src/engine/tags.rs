use crate::config::{SourceConfig, TagConfig, ValueExtraction, Condition, GameTag};
use std::collections::HashMap;
use regex::Regex;

pub fn extract_tags(
    item: &HashMap<String, String>,
    config: &SourceConfig,
) -> Vec<GameTag> {
    let mut tags = Vec::new();

    if let Some(tags_config) = &config.tags {
        for (tag_id, tag_config) in tags_config {
            // Check condition if present
            if let Some(condition) = &tag_config.condition {
                if !check_condition(item, condition) {
                    continue;
                }
            }

            // Extract label value
            let label = extract_tag_label(item, tag_config);

            // Don't create tag if label is empty
            if label.is_empty() {
                continue;
            }

            // Create the tag
            let tag = GameTag {
                id: tag_id.clone(),
                label,
                color: tag_config.color.clone(),
                background: tag_config.background.clone(),
                icon: tag_config.icon.clone(),
                priority: tag_config.priority,
                style: tag_config.style.as_ref().map(|s| format!("{:?}", s).to_lowercase()),
            };

            tags.push(tag);
        }
    }

    // Sort by priority (highest first)
    tags.sort_by(|a, b| b.priority.cmp(&a.priority));

    tags
}

fn extract_tag_label(item: &HashMap<String, String>, tag_config: &TagConfig) -> String {
    if let Some(value_from) = &tag_config.value_from {
        match value_from {
            ValueExtraction::Static(text) => text.clone(),

            ValueExtraction::FromField { field, prefix, suffix } => {
                let value = item.get(field).cloned().unwrap_or_default();

                if value.is_empty() {
                    return String::new();
                }

                let mut result = value;

                if let Some(pre) = prefix {
                    result = format!("{}{}", pre, result);
                }
                if let Some(suf) = suffix {
                    result = format!("{}{}", result, suf);
                }

                result
            }

            ValueExtraction::Template { template, fields } => {
                let mut result = template.clone();
                let mut has_empty = false;

                for field in fields {
                    let placeholder = format!("{{{}}}", field);
                    let field_value = item.get(field).cloned().unwrap_or_default();

                    if field_value.is_empty() {
                        has_empty = true;
                    }

                    result = result.replace(&placeholder, &field_value);
                }

                // If a field is empty, don't create the tag
                if has_empty {
                    return String::new();
                }

                result
            }
        }
    } else {
        tag_config.label.clone()
    }
}

fn check_condition(item: &HashMap<String, String>, condition: &Condition) -> bool {
    match condition {
        Condition::Contains { field, contains } => {
            item.get(field)
                .map(|v| v.contains(contains))
                .unwrap_or(false)
        }
        Condition::Regex { field, pattern } => {
            if let Some(value) = item.get(field) {
                if let Ok(re) = Regex::new(pattern) {
                    re.is_match(value)
                } else {
                    false
                }
            } else {
                false
            }
        }
        Condition::Equals { field, equals } => {
            item.get(field)
                .map(|v| v == equals)
                .unwrap_or(false)
        }
        Condition::NotEmpty { field } => {
            item.get(field)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
        }
        Condition::And { conditions } => {
            conditions.iter().all(|cond| check_condition(item, cond))
        }
        Condition::Or { conditions } => {
            conditions.iter().any(|cond| check_condition(item, cond))
        }
    }
}