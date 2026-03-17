use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::link_resolution::ResolutionStep;

/// Global hosts configuration loaded from _hosts.yaml
#[derive(Debug, Clone, Deserialize, Default)]
pub struct HostsConfig {
    #[serde(default)]
    pub hosts: HashMap<String, HostConfig>,
}

/// Configuration for a single file hosting service
#[derive(Debug, Clone, Deserialize)]
pub struct HostConfig {
    /// URL patterns to match this host (domain names or regex)
    pub patterns: Vec<String>,

    /// Display settings for UI
    pub display: HostDisplay,

    /// If true, this host cannot be downloaded directly (fallback to browser)
    #[serde(default)]
    pub browser_only: bool,

    /// Reason why browser is required (shown to user)
    #[serde(default)]
    pub browser_only_reason: Option<String>,

    /// Download method: "direct" (default), "webview", or "api"
    #[serde(default)]
    pub download_method: DownloadMethod,

    /// WebView configuration for hosts requiring JavaScript
    #[serde(default)]
    pub webview_config: Option<WebViewDownloadConfig>,

    /// Resolution steps to get direct download URL
    #[serde(default)]
    pub resolver: Option<HostResolver>,

    /// Error conditions to detect failed downloads
    #[serde(default)]
    pub error_conditions: Vec<ErrorCondition>,
}

/// Download method for a host
#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadMethod {
    #[default]
    Direct,
    Webview,
    Api,
}

/// Configuration for WebView-based downloads
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct WebViewDownloadConfig {
    /// CSS selector to wait for before proceeding
    #[serde(default)]
    pub wait_for: Option<String>,

    /// Maximum time to wait for element (ms)
    #[serde(default = "default_wait_timeout")]
    pub wait_timeout_ms: u64,

    /// CSS selector of button/link to click to start download
    #[serde(default)]
    pub click: Option<String>,

    /// If true, just intercept any download that starts (no clicking needed)
    #[serde(default)]
    pub intercept_download: bool,

    /// JavaScript to execute before clicking (e.g., close popups)
    #[serde(default)]
    pub pre_script: Option<String>,

    /// JavaScript to execute to extract download URL directly
    #[serde(default)]
    pub extract_url_script: Option<String>,

    /// Regex pattern to match download URLs (for interception)
    #[serde(default)]
    pub download_url_pattern: Option<String>,
}

fn default_wait_timeout() -> u64 {
    30000 // 30 seconds default
}

/// Display configuration for a host
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HostDisplay {
    /// Pretty name to show in UI (e.g., "GoFile")
    pub label: String,

    /// Icon name for lucide-svelte
    #[serde(default)]
    pub icon: Option<String>,

    /// Accent color for the button
    #[serde(default)]
    pub color: Option<String>,
}

/// Resolver configuration for a host
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct HostResolver {
    /// Resolution steps to execute
    pub steps: Vec<ResolutionStep>,

    /// Optional headers to add to all requests
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,

    /// Whether authentication cookies are required
    #[serde(default)]
    pub requires_auth: bool,
}

/// Condition to detect download errors
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorCondition {
    /// Regex pattern to match in response body
    #[serde(default)]
    pub pattern: Option<String>,

    /// CSS selector to check for presence
    #[serde(default)]
    pub selector: Option<String>,

    /// Text that must be contained in matched element
    #[serde(default)]
    pub contains: Option<String>,

    /// Type of error (maps to DownloadErrorType)
    pub error_type: String,

    /// Human-readable error message
    pub message: String,
}

/// Result of host detection
#[derive(Debug, Clone, Serialize)]
pub struct DetectedHost {
    /// Internal host identifier
    pub host_id: String,

    /// Pretty display name
    pub label: String,

    /// Icon name for UI
    pub icon: Option<String>,

    /// Accent color
    pub color: Option<String>,

    /// Whether direct download is supported
    pub supports_direct_download: bool,

    /// Reason if browser-only
    pub browser_only_reason: Option<String>,
}

/// Result of a smart download attempt
#[derive(Debug, Clone, Serialize)]
pub struct SmartDownloadResult {
    pub success: bool,
    pub file_path: Option<String>,
    pub file_size: Option<u64>,  // Size in bytes
    pub error: Option<DownloadError>,
    pub fallback_url: Option<String>,
}

/// Download error details
#[derive(Debug, Clone, Serialize)]
pub struct DownloadError {
    pub error_type: DownloadErrorType,
    pub message: String,
    pub recoverable: bool,
}

/// Types of download errors
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadErrorType {
    CaptchaRequired,
    PremiumOnly,
    GeoBlocked,
    RateLimited,
    NetworkError,
    ResolutionFailed,
    UnsupportedHost,
    FileNotFound,
    BrowserRequired,
    Unknown,
}

impl From<&str> for DownloadErrorType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "captcha_required" | "captcha" => Self::CaptchaRequired,
            "premium_only" | "premium" => Self::PremiumOnly,
            "geo_blocked" | "geo" => Self::GeoBlocked,
            "rate_limited" | "rate_limit" => Self::RateLimited,
            "network_error" | "network" => Self::NetworkError,
            "resolution_failed" | "resolution" => Self::ResolutionFailed,
            "unsupported_host" | "unsupported" => Self::UnsupportedHost,
            "file_not_found" | "not_found" | "404" => Self::FileNotFound,
            "browser_required" | "browser" => Self::BrowserRequired,
            _ => Self::Unknown,
        }
    }
}
