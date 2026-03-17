use crate::config::{SourceConfig, Transformation, UrlRule};
use std::collections::HashMap;
use regex::Regex;

pub fn apply_transformations(
    mut item: HashMap<String, String>,
    config: &SourceConfig,
) -> Result<HashMap<String, String>, String> {
    if let Some(transformations) = &config.transformations {
        for (field_name, transforms) in transformations {
            if let Some(value) = item.get(field_name).cloned() {
                let transformed = apply_transform_chain(value, transforms, &item, config)?;
                item.insert(field_name.clone(), transformed);
            }
        }
    }

    Ok(item)
}

fn apply_transform_chain(
    mut value: String,
    transforms: &[Transformation],
    item: &HashMap<String, String>,
    config: &SourceConfig,
) -> Result<String, String> {
    for transform in transforms {
        value = apply_single_transform(value, transform, item, config)?;
    }
    Ok(value)
}

fn apply_single_transform(
    value: String,
    transform: &Transformation,
    item: &HashMap<String, String>,
    config: &SourceConfig,
) -> Result<String, String> {
    let result = match transform {
        Transformation::Replace { pattern, replacement, regex } => {
            if *regex {
                let re = Regex::new(pattern)
                    .map_err(|e| format!("Invalid regex: {}", e))?;
                re.replace_all(&value, replacement.as_str()).to_string()
            } else {
                value.replace(pattern, replacement)
            }
        }

        Transformation::Trim => value.trim().to_string(),

        Transformation::Lowercase => value.to_lowercase(),

        Transformation::Uppercase => value.to_uppercase(),

        Transformation::Capitalize => {
            let mut chars = value.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        }

        Transformation::CollapseWhitespace => {
            value.split_whitespace().collect::<Vec<_>>().join(" ")
        }

        Transformation::Truncate { max_length, suffix } => {
            if value.len() > *max_length {
                format!("{}{}", &value[..*max_length], suffix)
            } else {
                value
            }
        }

        Transformation::RemoveHtml => {
            let re = Regex::new(r"<[^>]+>").unwrap();
            re.replace_all(&value, "").to_string()
        }

        Transformation::StripPrefix { prefix } => {
            value.strip_prefix(prefix).unwrap_or(&value).to_string()
        }

        Transformation::StripSuffix { suffix } => {
            value.strip_suffix(suffix).unwrap_or(&value).to_string()
        }

        Transformation::UrlNormalize { base_url, rules } => {
            normalize_url(&value, base_url.as_deref().unwrap_or(&config.base_url), rules)
        }

        Transformation::Template { template } => {
            apply_template(template, item)
        }

        Transformation::Extract { pattern, group } => {
            let re = Regex::new(pattern)
                .map_err(|e| format!("Invalid regex: {}", e))?;

            if let Some(captures) = re.captures(&value) {
                captures.get(*group)
                    .map(|m: regex::Match| m.as_str().to_string())
                    .unwrap_or(value)
            } else {
                value
            }
        }

        Transformation::Split { delimiter, index, join } => {
            let parts: Vec<&str> = value.split(delimiter.as_str()).collect();

            if let Some(idx) = index {
                parts.get(*idx).map(|s| s.to_string()).unwrap_or(value)
            } else if let Some(join_str) = join {
                parts.join(join_str)
            } else {
                value
            }
        }

        Transformation::Append { text } => {
            format!("{}{}", value, text)
        }

        Transformation::Prepend { text } => {
            format!("{}{}", text, value)
        }

        Transformation::Default { value: default } => {
            if value.is_empty() {
                default.clone()
            } else {
                value
            }
        }

        Transformation::Custom { _name, _params } => {
            let _ = (_name, _params);
            value
        }
    };

    Ok(result)
}

fn normalize_url(url: &str, base_url: &str, rules: &[UrlRule]) -> String {
    let mut result = url.trim().to_string();

    for rule in rules {
        result = match rule {
            UrlRule::PrependProtocol(protocol) => {
                if result.starts_with("//") {
                    format!("{}{}", protocol, result)
                } else {
                    result
                }
            }
            UrlRule::PrependDomain(domain) => {
                if result.starts_with("/") && !result.starts_with("//") {
                    format!("{}{}", domain, result)
                } else {
                    result
                }
            }
            UrlRule::RemoveQueryParams => {
                result.split('?').next().unwrap_or(&result).to_string()
            }
            UrlRule::RemoveFragment => {
                result.split('#').next().unwrap_or(&result).to_string()
            }
        };
    }

    // Final normalization if needed
    if !result.starts_with("http") && !result.is_empty() {
        if result.starts_with("//") {
            format!("https:{}", result)
        } else if result.starts_with("/") {
            format!("{}{}", base_url, result)
        } else {
            result
        }
    } else {
        result
    }
}

fn apply_template(template: &str, item: &HashMap<String, String>) -> String {
    let mut result = template.to_string();

    for (key, value) in item {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, value);
    }

    result
}