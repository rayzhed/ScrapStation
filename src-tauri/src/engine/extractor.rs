use crate::config::{SourceConfig, MetadataExtractor, Condition, ExtractionMethod, ListExtractionMethod, CustomFieldExtractor, FieldSource, Transformation};
use std::collections::HashMap;
use regex::Regex;
use chrono::NaiveDate;

pub fn extract_metadata(
    mut item: HashMap<String, String>,
    config: &SourceConfig,
) -> Result<HashMap<String, String>, String> {
    if let Some(metadata_extractors) = &config.metadata_extraction {
        for (field_name, extractor) in metadata_extractors {
            if let Some(value) = extract_metadata_field(&item, extractor)? {
                item.insert(field_name.clone(), value);
            }
        }
    }

    Ok(item)
}

fn extract_metadata_field(
    item: &HashMap<String, String>,
    extractor: &MetadataExtractor,
) -> Result<Option<String>, String> {
    match extractor {
        MetadataExtractor::Boolean { condition } => {
            if let Some(cond) = condition {
                let result = check_condition(item, cond);
                Ok(Some(result.to_string()))
            } else {
                Ok(Some("false".to_string()))
            }
        }

        MetadataExtractor::String { extraction, transformations, fallback } => {
            let mut value = if let Some(method) = extraction {
                extract_string(item, method)?
            } else {
                String::new()
            };

            if let Some(transforms) = transformations {
                for transform in transforms {
                    value = apply_string_transform(value, transform, item)?;
                }
            }

            if value.is_empty() {
                Ok(fallback.clone())
            } else {
                Ok(Some(value))
            }
        }

        MetadataExtractor::Date { extraction, input_format, output_format, fallback } => {
            let raw_date = if let Some(method) = extraction {
                extract_string(item, method)?
            } else {
                String::new()
            };

            if raw_date.is_empty() {
                return Ok(fallback.clone());
            }

            let formatted = format_date(&raw_date, input_format.as_deref(), output_format.as_deref())
                .unwrap_or_else(|| fallback.clone().unwrap_or(raw_date));

            Ok(Some(formatted))
        }

        MetadataExtractor::List { extraction, join_with } => {
            let values = if let Some(method) = extraction {
                extract_list(item, method)?
            } else {
                Vec::new()
            };

            if values.is_empty() {
                Ok(None)
            } else {
                let joined = values.join(join_with.as_deref().unwrap_or(", "));
                Ok(Some(joined))
            }
        }

        MetadataExtractor::Number { extraction, fallback } => {
            let value = if let Some(method) = extraction {
                extract_string(item, method)?
            } else {
                String::new()
            };

            if let Ok(num) = value.parse::<f64>() {
                Ok(Some(num.to_string()))
            } else {
                Ok(fallback.map(|f| f.to_string()))
            }
        }

        MetadataExtractor::Custom { _config } => {
            let _ = _config;
            Ok(None)
        }
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

fn extract_string(item: &HashMap<String, String>, method: &ExtractionMethod) -> Result<String, String> {
    match method {
        ExtractionMethod::Pattern { pattern, in_field } => {
            let source = get_source_field(item, in_field)?;

            if let Some(start_idx) = source.find(pattern) {
                let after_pattern = &source[start_idx + pattern.len()..];
                let next_word = after_pattern
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                Ok(next_word)
            } else {
                Ok(String::new())
            }
        }

        ExtractionMethod::Regex { pattern, in_field, group } => {
            let source = get_source_field(item, in_field)?;
            let re = Regex::new(pattern)
                .map_err(|e| format!("Invalid regex '{}': {}", pattern, e))?;

            if let Some(captures) = re.captures(&source) {
                Ok(captures.get(*group)
                    .map(|m: regex::Match| m.as_str().to_string())
                    .unwrap_or_default())
            } else {
                Ok(String::new())
            }
        }

        ExtractionMethod::NextWord { after_pattern, in_field } => {
            let source = get_source_field(item, in_field)?;

            if let Some(start_idx) = source.find(after_pattern) {
                let after = &source[start_idx + after_pattern.len()..];
                let next_word = after
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                Ok(next_word)
            } else {
                Ok(String::new())
            }
        }

        ExtractionMethod::Between { start, end, in_field } => {
            let source = get_source_field(item, in_field)?;

            if let Some(start_idx) = source.find(start) {
                let after_start = &source[start_idx + start.len()..];
                if let Some(end_idx) = after_start.find(end) {
                    return Ok(after_start[..end_idx].to_string());
                }
            }
            Ok(String::new())
        }

        ExtractionMethod::FromField { field } => {
            Ok(item.get(field).cloned().unwrap_or_default())
        }

        ExtractionMethod::JsonPath { path } => {
            let _ = path;
            Ok(String::new())
        }

        ExtractionMethod::Custom { _name, _params } => {
            let _ = (_name, _params);
            Ok(String::new())
        }
    }
}

fn extract_list(item: &HashMap<String, String>, method: &ListExtractionMethod) -> Result<Vec<String>, String> {
    match method {
        ListExtractionMethod::Keywords { in_field, keywords } => {
            let source = get_source_field(item, in_field)?;
            let mut results = Vec::new();

            for keyword in keywords {
                if keyword.regex {
                    if let Ok(re) = Regex::new(&keyword.pattern) {
                        if re.is_match(&source) {
                            results.push(keyword.value.clone());
                        }
                    }
                } else if source.contains(&keyword.pattern) {
                    results.push(keyword.value.clone());
                }
            }

            Ok(results)
        }

        ListExtractionMethod::Split { in_field, delimiter } => {
            let source = item.get(in_field).cloned().unwrap_or_default();
            Ok(source.split(delimiter).map(|s| s.trim().to_string()).collect())
        }

        ListExtractionMethod::Regex { in_field, pattern } => {
            let source = get_source_field(item, in_field)?;
            let re = Regex::new(pattern)
                .map_err(|e| format!("Invalid regex: {}", e))?;

            Ok(re.find_iter(&source)
                .map(|m| m.as_str().to_string())
                .collect())
        }

        ListExtractionMethod::Custom { _name, _params } => {
            let _ = (_name, _params);
            Ok(Vec::new())
        }
    }
}

fn get_source_field(item: &HashMap<String, String>, field: &Option<String>) -> Result<String, String> {
    if let Some(field_name) = field {
        Ok(item.get(field_name).cloned().unwrap_or_default())
    } else {
        Ok(item.values().cloned().collect::<Vec<_>>().join(" "))
    }
}

fn apply_string_transform(
    value: String,
    transform: &Transformation,
    _item: &HashMap<String, String>,
) -> Result<String, String> {
    match transform {
        Transformation::Replace { pattern, replacement, regex } => {
            if *regex {
                let re = Regex::new(pattern)
                    .map_err(|e| format!("Invalid regex: {}", e))?;
                Ok(re.replace_all(&value, replacement.as_str()).to_string())
            } else {
                Ok(value.replace(pattern, replacement))
            }
        }
        Transformation::Trim => Ok(value.trim().to_string()),
        Transformation::Default { value: default } => {
            if value.is_empty() {
                Ok(default.clone())
            } else {
                Ok(value)
            }
        }
        _ => Ok(value),
    }
}

fn format_date(date_str: &str, input_format: Option<&str>, output_format: Option<&str>) -> Option<String> {
    let input_fmt = input_format.unwrap_or("%d.%m.%Y");
    let output_fmt = output_format.unwrap_or("%d %b %Y");

    if let Ok(parsed) = NaiveDate::parse_from_str(date_str, input_fmt) {
        Some(parsed.format(output_fmt).to_string())
    } else {
        None
    }
}

pub fn apply_custom_fields(
    mut item: HashMap<String, String>,
    config: &SourceConfig,
) -> Result<HashMap<String, String>, String> {
    if let Some(custom_fields) = &config.custom_fields {
        for (field_name, extractor) in custom_fields {
            if let Some(value) = extract_custom_field(&item, extractor)? {
                item.insert(field_name.clone(), value);
            }
        }
    }

    Ok(item)
}

fn extract_custom_field(
    item: &HashMap<String, String>,
    extractor: &CustomFieldExtractor,
) -> Result<Option<String>, String> {
    // Check condition if present
    if let Some(condition) = &extractor.condition {
        if !check_condition(item, condition) {
            return Ok(None);
        }
    }

    let mut value = match &extractor.source {
        FieldSource::Static(text) => text.clone(),

        FieldSource::FromField { field } => {
            item.get(field).cloned().unwrap_or_default()
        }

        FieldSource::Computed { template, fields } => {
            let mut result = template.clone();
            for field in fields {
                let placeholder = format!("{{{}}}", field);
                let field_value = item.get(field).cloned().unwrap_or_default();
                result = result.replace(&placeholder, &field_value);
            }
            result
        }

        FieldSource::Json { value: json_val } => {
            json_val.to_string()
        }
    };

    if let Some(transforms) = &extractor.transformations {
        for transform in transforms {
            value = apply_string_transform(value, transform, item)?;
        }
    }

    Ok(Some(value))
}