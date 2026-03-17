use crate::config::JsonPathsConfig;
use serde_json::Value;
use std::collections::HashMap;

pub fn parse(
    json_str: &str,
    config: &JsonPathsConfig,
) -> Result<Vec<HashMap<String, String>>, String> {
    let json: Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let items_path = &config.items;
    let items_array = get_json_path(&json, items_path)
        .and_then(|v| v.as_array())
        .ok_or_else(|| format!("Could not find array at path: {}", items_path))?;

    log::info!("Found {} items in JSON", items_array.len());

    let mut results = Vec::new();

    for (index, item) in items_array.iter().enumerate() {
        let mut extracted: HashMap<String, String> = HashMap::new();

        for (field_name, json_path) in &config.fields {
            if let Some(value) = get_json_path(item, json_path) {
                let string_value = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => value.to_string(),
                };
                extracted.insert(field_name.clone(), string_value);
            }
        }

        if !extracted.is_empty() {
            log::info!("Item {}: {} fields extracted", index + 1, extracted.len());
            results.push(extracted);
        }
    }

    Ok(results)
}

fn get_json_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    // "$" or empty string means the root value itself (e.g. a top-level JSON array)
    if path.is_empty() || path == "$" {
        return Some(value);
    }

    let mut current = value;

    for part in path.split('.') {
        current = if part.contains('[') && part.contains(']') {
            let (key, index_str) = part.split_once('[')?;
            let index = index_str.trim_end_matches(']').parse::<usize>().ok()?;
            current.get(key)?.get(index)?
        } else {
            current.get(part)?
        };
    }

    Some(current)
}