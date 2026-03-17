use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use once_cell::sync::Lazy;

/// User settings stored per source
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserSettings {
    /// Settings values per source ID -> setting ID -> value
    #[serde(default)]
    pub source_settings: HashMap<String, HashMap<String, JsonValue>>,
}

/// Global settings instance
static SETTINGS: Lazy<Mutex<UserSettings>> = Lazy::new(|| {
    Mutex::new(UserSettings::load().unwrap_or_default())
});

impl UserSettings {
    /// Get the settings file path in the user's config directory
    fn settings_path() -> PathBuf {
        // Use the proper config directory to avoid triggering hot reload
        if let Some(config_dir) = dirs::config_dir() {
            let app_dir = config_dir.join(crate::constants::APP_NAME);
            // Create the directory if it doesn't exist
            if !app_dir.exists() {
                let _ = std::fs::create_dir_all(&app_dir);
            }
            app_dir.join("user_settings.json")
        } else {
            // Fallback to current directory if config dir is not available
            let mut path = std::env::current_dir().unwrap_or_default();
            path.push("user_settings.json");
            path
        }
    }

    /// Load settings from file
    pub fn load() -> Result<Self, String> {
        let path = Self::settings_path();

        if !path.exists() {
            log::info!("No user settings file found, using defaults");
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read settings: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse settings: {}", e))
    }

    /// Save settings to file
    pub fn save(&self) -> Result<(), String> {
        let path = Self::settings_path();
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write settings: {}", e))?;

        log::info!("Settings saved to {:?}", path);
        Ok(())
    }

    // ===== Generic Settings API =====

    /// Get a specific setting value for a source
    pub fn get_setting(source_id: &str, setting_id: &str) -> Option<JsonValue> {
        let settings = SETTINGS.lock().ok()?;
        settings.source_settings
            .get(source_id)
            .and_then(|s| s.get(setting_id))
            .cloned()
    }

    /// Get all settings for a source
    pub fn get_source_settings(source_id: &str) -> HashMap<String, JsonValue> {
        if let Ok(settings) = SETTINGS.lock() {
            settings.source_settings
                .get(source_id)
                .cloned()
                .unwrap_or_default()
        } else {
            HashMap::new()
        }
    }

    /// Set a specific setting value for a source
    pub fn set_setting(source_id: &str, setting_id: &str, value: JsonValue) -> Result<(), String> {
        let mut settings = SETTINGS.lock()
            .map_err(|e| format!("Failed to lock settings: {}", e))?;

        settings.source_settings
            .entry(source_id.to_string())
            .or_insert_with(HashMap::new)
            .insert(setting_id.to_string(), value);

        settings.save()?;

        log::info!("Setting '{}' updated for source: {}", setting_id, source_id);
        Ok(())
    }

    /// Set multiple settings at once for a source
    pub fn set_source_settings(source_id: &str, values: HashMap<String, JsonValue>) -> Result<(), String> {
        let mut settings = SETTINGS.lock()
            .map_err(|e| format!("Failed to lock settings: {}", e))?;

        settings.source_settings.insert(source_id.to_string(), values);
        settings.save()?;

        log::info!("All settings updated for source: {}", source_id);
        Ok(())
    }

    /// Clear a specific setting for a source
    pub fn clear_setting(source_id: &str, setting_id: &str) -> Result<(), String> {
        let mut settings = SETTINGS.lock()
            .map_err(|e| format!("Failed to lock settings: {}", e))?;

        if let Some(source_settings) = settings.source_settings.get_mut(source_id) {
            source_settings.remove(setting_id);
        }
        settings.save()?;

        log::info!("Setting '{}' cleared for source: {}", setting_id, source_id);
        Ok(())
    }

    /// Clear all settings for a source
    pub fn clear_source_settings(source_id: &str) -> Result<(), String> {
        let mut settings = SETTINGS.lock()
            .map_err(|e| format!("Failed to lock settings: {}", e))?;

        settings.source_settings.remove(source_id);
        settings.save()?;

        log::info!("All settings cleared for source: {}", source_id);
        Ok(())
    }

    // ===== Backward Compatibility for Cookies =====

    /// Get cookies for a specific source (reads from 'cookies' setting)
    pub fn get_cookies(source_id: &str) -> Option<String> {
        Self::get_setting(source_id, "cookies")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
    }

    /// Set cookies for a specific source (writes to 'cookies' setting)
    pub fn set_cookies(source_id: &str, cookies: &str) -> Result<(), String> {
        Self::set_setting(source_id, "cookies", JsonValue::String(cookies.to_string()))
    }

    /// Clear cookies for a specific source
    pub fn clear_cookies(source_id: &str) -> Result<(), String> {
        Self::clear_setting(source_id, "cookies")
    }
}
