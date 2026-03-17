use crate::engine::SourceLoader;
use crate::settings::UserSettings;
use std::collections::HashMap;

#[tauri::command]
pub async fn get_source_settings_schema(
    source_id: String,
) -> Result<Vec<crate::config::SettingDefinition>, String> {
    let config = SourceLoader::load_by_id(&source_id)?;
    Ok(config.settings.unwrap_or_default())
}

#[tauri::command]
pub async fn get_source_settings_values(
    source_id: String,
) -> Result<HashMap<String, serde_json::Value>, String> {
    Ok(UserSettings::get_source_settings(&source_id))
}

#[tauri::command]
pub async fn set_source_setting(
    source_id: String,
    setting_id: String,
    value: serde_json::Value,
) -> Result<(), String> {
    UserSettings::set_setting(&source_id, &setting_id, value)
}

#[tauri::command]
pub async fn set_source_settings(
    source_id: String,
    values: HashMap<String, serde_json::Value>,
) -> Result<(), String> {
    UserSettings::set_source_settings(&source_id, values)
}

#[tauri::command]
pub async fn clear_source_settings(source_id: String) -> Result<(), String> {
    UserSettings::clear_source_settings(&source_id)
}

#[tauri::command]
pub async fn get_setting_sections(source_id: String) -> Result<serde_json::Value, String> {
    let config = SourceLoader::load_by_id(&source_id)?;

    if let Some(sections) = config.setting_sections {
        serde_json::to_value(sections).map_err(|e| format!("Failed to serialize sections: {}", e))
    } else {
        Ok(serde_json::json!([]))
    }
}

#[tauri::command]
pub async fn check_storage_condition(
    source_id: String,
    key: String,
) -> Result<serde_json::Value, String> {
    let value = UserSettings::get_setting(&source_id, &key);

    Ok(serde_json::json!({
        "exists": value.is_some(),
        "value": value
    }))
}
