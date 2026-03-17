use regex::Regex;
use url::Url;

use crate::config::hosts::{DetectedHost, HostConfig, HostsConfig};

/// Detect the host from a URL using the provided hosts config
pub fn detect_host(url: &str, hosts_config: Option<&HostsConfig>) -> DetectedHost {
    // Try to parse the URL
    let parsed = match Url::parse(url) {
        Ok(u) => u,
        Err(_) => return create_unknown_host(url),
    };

    let domain = match parsed.host_str() {
        Some(d) => d.to_lowercase(),
        None => return create_unknown_host(url),
    };

    // Try to match against configured hosts if config provided
    if let Some(config) = hosts_config {
        for (host_id, host_config) in &config.hosts {
            if matches_host(&domain, &host_config.patterns) {
                return DetectedHost {
                    host_id: host_id.clone(),
                    label: host_config.display.label.clone(),
                    icon: host_config.display.icon.clone(),
                    color: host_config.display.color.clone(),
                    supports_direct_download: !host_config.browser_only && host_config.resolver.is_some(),
                    browser_only_reason: host_config.browser_only_reason.clone(),
                };
            }
        }
    }

    // No match found - auto-generate from domain
    create_auto_detected_host(&domain)
}

/// Check if a domain matches any of the patterns
fn matches_host(domain: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        // Check if pattern looks like a regex (contains regex special chars)
        if pattern.contains('*') || pattern.contains('^') || pattern.contains('$') || pattern.contains('(') {
            // Treat as regex
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(domain) {
                    return true;
                }
            }
        } else {
            // Treat as domain suffix match
            // "gofile.io" should match "gofile.io" and "www.gofile.io"
            if domain == pattern || domain.ends_with(&format!(".{}", pattern)) {
                return true;
            }
        }
    }
    false
}

/// Create a DetectedHost for an unknown URL
fn create_unknown_host(url: &str) -> DetectedHost {
    DetectedHost {
        host_id: "unknown".to_string(),
        label: extract_domain_label(url).unwrap_or_else(|| "Download".to_string()),
        icon: None,
        color: None,
        supports_direct_download: false,
        browser_only_reason: None,
    }
}

/// Create an auto-detected host from domain
fn create_auto_detected_host(domain: &str) -> DetectedHost {
    let label = domain_to_label(domain);

    DetectedHost {
        host_id: format!("auto:{}", domain),
        label,
        icon: None,
        color: None,
        supports_direct_download: false, // Unknown hosts don't support direct download
        browser_only_reason: None,
    }
}

/// Convert a domain to a nice label
/// e.g., "gofile.io" -> "GoFile", "mega.nz" -> "Mega"
fn domain_to_label(domain: &str) -> String {
    // Remove common prefixes
    let domain = domain
        .strip_prefix("www.")
        .or_else(|| domain.strip_prefix("dl."))
        .or_else(|| domain.strip_prefix("download."))
        .unwrap_or(domain);

    // Get the main part (before the TLD)
    let main_part = domain
        .split('.')
        .next()
        .unwrap_or(domain);

    // Capitalize first letter and make rest lowercase
    capitalize_label(main_part)
}

/// Capitalize a label from a raw domain segment.
/// All site-specific display names live in the YAML `hosts:` config.
/// This function is only a generic fallback for hosts not present in any YAML.
fn capitalize_label(s: &str) -> String {
    if s.is_empty() {
        return s.to_string();
    }
    // Capitalize first letter, keep rest as-is
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Extract domain label from a URL string
fn extract_domain_label(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let domain = parsed.host_str()?;
    Some(domain_to_label(domain))
}

/// Get host config by ID from the provided config
pub fn get_host_config<'a>(host_id: &str, hosts_config: Option<&'a HostsConfig>) -> Option<&'a HostConfig> {
    hosts_config?.hosts.get(host_id)
}

/// Detect host and get its full config
pub fn detect_host_with_config<'a>(
    url: &str,
    hosts_config: Option<&'a HostsConfig>,
) -> (DetectedHost, Option<&'a HostConfig>) {
    let detected = detect_host(url, hosts_config);
    let config = if detected.host_id.starts_with("auto:") || detected.host_id == "unknown" {
        None
    } else {
        get_host_config(&detected.host_id, hosts_config)
    };
    (detected, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_to_label() {
        assert_eq!(domain_to_label("gofile.io"), "GoFile");
        assert_eq!(domain_to_label("mega.nz"), "MEGA");
        assert_eq!(domain_to_label("www.mediafire.com"), "MediaFire");
        assert_eq!(domain_to_label("pixeldrain.com"), "PixelDrain");
        assert_eq!(domain_to_label("example.com"), "Example");
    }

    #[test]
    fn test_matches_host() {
        assert!(matches_host("gofile.io", &["gofile.io".to_string()]));
        assert!(matches_host("www.gofile.io", &["gofile.io".to_string()]));
        assert!(!matches_host("notgofile.io", &["gofile.io".to_string()]));
    }
}
