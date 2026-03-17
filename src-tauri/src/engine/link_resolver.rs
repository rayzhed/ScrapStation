use crate::config::{LinkResolutionConfig, LinkResolver, ResolutionStep, ExtractionMethodType, Transformation};
use crate::utils::http_client::HttpClient;
use std::collections::HashMap;
use scraper::{Html, Selector};
use regex::Regex;

pub struct LinkResolverEngine;

impl LinkResolverEngine {
    /// Resolve links for all items according to configuration
    pub async fn resolve_links(
        items: Vec<HashMap<String, String>>,
        config: &LinkResolutionConfig,
        http_client: &HttpClient,
    ) -> Result<Vec<HashMap<String, String>>, String> {
        if !config.enabled {
            log::debug!("[LinkResolver] Resolution disabled, skipping");
            return Ok(items);
        }

        log::debug!("[LinkResolver] Starting resolution for {} items", items.len());
        let mut resolved_items = Vec::new();

        for mut item in items {
            for (output_field, resolver) in &config.resolvers {
                log::debug!("[LinkResolver] Processing resolver '{}' with source_field '{}'", output_field, resolver.source_field);
                if let Some(source_url) = item.get(&resolver.source_field).cloned() {
                    log::debug!("[LinkResolver] Found source URL: {}", source_url);
                    match Self::resolve_single(&source_url, resolver, http_client, &item).await {
                        Ok(resolved_data) => {
                            log::info!("[LinkResolver] Successfully resolved to: {}", resolved_data.url);
                            item.insert(output_field.clone(), resolved_data.url);
                            for (key, value) in resolved_data.metadata {
                                item.insert(key, value);
                            }
                        }
                        Err(e) => {
                            log::error!("[LinkResolver] Resolution failed: {}", e);
                        }
                    }
                } else {
                    log::warn!("[LinkResolver] Source field '{}' not found in item", resolver.source_field);
                }
            }
            resolved_items.push(item);
        }

        Ok(resolved_items)
    }

    async fn resolve_single(
        url: &str,
        resolver: &LinkResolver,
        http_client: &HttpClient,
        _item: &HashMap<String, String>,
    ) -> Result<ResolvedData, String> {
        let mut current_value = url.to_string();
        let mut context = ResolutionContext {
            current_content: String::new(),
            metadata: HashMap::new(),
        };

        for step in resolver.steps.iter() {
            current_value = Self::execute_step(
                current_value,
                step,
                http_client,
                &mut context,
            ).await?;
        }

        Ok(ResolvedData {
            url: current_value,
            metadata: context.metadata,
        })
    }

    async fn execute_step(
        value: String,
        step: &ResolutionStep,
        http_client: &HttpClient,
        context: &mut ResolutionContext,
    ) -> Result<String, String> {
        match step {
            ResolutionStep::Fetch { follow_redirects, headers, timeout_ms } => {
                Self::fetch_step(&value, *follow_redirects, headers, *timeout_ms, http_client, context).await
            }

            ResolutionStep::Extract { method, pattern, group, selector, attribute, fallback } => {
                Self::extract_step(&context.current_content, method, pattern, group, selector, attribute, fallback)
            }

            ResolutionStep::Transform { transformations } => {
                Self::transform_step(value, transformations)
            }

            ResolutionStep::Wait { duration_ms } => {
                Self::wait_step(*duration_ms).await
            }

            ResolutionStep::Custom { _name, _params } => {
                let _ = (_name, _params);
                Ok(value)
            }
        }
    }

    async fn fetch_step(
        url: &str,
        _follow_redirects: bool,
        headers: &Option<HashMap<String, String>>,
        _timeout_ms: Option<u64>,
        http_client: &HttpClient,
        context: &mut ResolutionContext,
    ) -> Result<String, String> {
        log::debug!("[LinkResolver] Fetch step - URL: {}", url);
        if let Some(h) = headers {
            log::debug!("[LinkResolver] Fetch headers: {:?}", h);
        }

        let content = http_client.get_with_headers(url, headers).await
            .map_err(|e| {
                log::error!("[LinkResolver] Fetch failed: {}", e);
                format!("Fetch failed: {}", e)
            })?;

        log::debug!("[LinkResolver] Fetched {} bytes", content.len());
        // Log first 500 chars to see what we got
        log::debug!("[LinkResolver] Content preview: {}", &content.chars().take(500).collect::<String>());

        context.current_content = content;
        Ok(url.to_string())
    }

    fn extract_step(
        content: &str,
        method: &ExtractionMethodType,
        pattern: &Option<String>,
        group: &Option<usize>,
        selector: &Option<String>,
        attribute: &Option<String>,
        fallback: &Option<String>,
    ) -> Result<String, String> {
        let result = match method {
            ExtractionMethodType::Regex => {
                Self::extract_with_regex(content, pattern, group)?
            }

            ExtractionMethodType::Selector => {
                Self::extract_with_selector(content, selector, attribute, pattern, group)?
            }

            ExtractionMethodType::Text => {
                content.to_string()
            }

            ExtractionMethodType::JsonPath => {
                return Err("JsonPath extraction not yet implemented".to_string());
            }

            ExtractionMethodType::XPath => {
                return Err("XPath extraction not yet implemented".to_string());
            }
        };

        if result.is_empty() {
            if let Some(fb) = fallback {
                return Ok(fb.clone());
            } else {
                return Err("Extraction returned empty result and no fallback provided".to_string());
            }
        }

        Ok(result)
    }

    fn extract_with_regex(
        content: &str,
        pattern: &Option<String>,
        group: &Option<usize>,
    ) -> Result<String, String> {
        let pat = pattern.as_ref()
            .ok_or_else(|| "Regex extraction requires 'pattern' parameter".to_string())?;

        log::debug!("[LinkResolver] Regex extraction - pattern: {}", pat);
        log::debug!("[LinkResolver] Content length: {} chars", content.len());

        let re = Regex::new(pat)
            .map_err(|e| {
                log::error!("[LinkResolver] Invalid regex '{}': {}", pat, e);
                format!("Invalid regex '{}': {}", pat, e)
            })?;

        if let Some(captures) = re.captures(content) {
            let grp = group.unwrap_or(1);
            match captures.get(grp) {
                Some(m) => {
                    log::info!("[LinkResolver] Regex matched! Group {}: {}", grp, m.as_str());
                    Ok(m.as_str().to_string())
                }
                None => {
                    log::error!("[LinkResolver] Regex group {} not found in captures", grp);
                    Err(format!("Regex group {} not found", grp))
                }
            }
        } else {
            log::error!("[LinkResolver] No match found for pattern: {}", pat);
            // Log a sample of the content to help debug
            if content.contains("location") {
                log::debug!("[LinkResolver] Content contains 'location', searching for context...");
                for line in content.lines() {
                    if line.contains("location") {
                        log::debug!("[LinkResolver] Found line with 'location': {}", line.chars().take(200).collect::<String>());
                    }
                }
            }
            Err(format!("No match found for pattern: {}", pat))
        }
    }

    fn extract_with_selector(
        content: &str,
        selector: &Option<String>,
        attribute: &Option<String>,
        pattern: &Option<String>,
        group: &Option<usize>,
    ) -> Result<String, String> {
        let sel = selector.as_ref()
            .ok_or_else(|| "Selector extraction requires 'selector' parameter".to_string())?;

        let document = Html::parse_document(content);
        let css_selector = Selector::parse(sel)
            .map_err(|e| format!("Invalid CSS selector '{}': {:?}", sel, e))?;

        let element = document.select(&css_selector).next()
            .ok_or_else(|| format!("Selector '{}' not found in content", sel))?;

        let extracted_value = if let Some(attr) = attribute {
            element.value().attr(attr)
                .ok_or_else(|| format!("Attribute '{}' not found on element", attr))?
                .to_string()
        } else {
            element.text().collect::<String>()
        };

        if let Some(pat) = pattern {
            Self::extract_with_regex(&extracted_value, &Some(pat.clone()), group)
        } else {
            Ok(extracted_value)
        }
    }

    fn transform_step(
        value: String,
        transformations: &[Transformation],
    ) -> Result<String, String> {
        let mut item = HashMap::new();
        item.insert("_value".to_string(), value);
        let mut current_value = item.get("_value")
            .ok_or_else(|| "missing _value field".to_string())?
            .clone();
        for transform in transformations {
            current_value = apply_single_transform(current_value, transform, &item)?;
        }

        Ok(current_value)
    }

    async fn wait_step(duration_ms: u64) -> Result<String, String> {
        tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
        Ok(String::new())
    }
}

struct ResolutionContext {
    current_content: String,
    metadata: HashMap<String, String>,
}

struct ResolvedData {
    url: String,
    metadata: HashMap<String, String>,
}

fn apply_single_transform(
    value: String,
    transform: &Transformation,
    _item: &HashMap<String, String>,
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

        Transformation::StripPrefix { prefix } => {
            value.strip_prefix(prefix).unwrap_or(&value).to_string()
        }

        Transformation::StripSuffix { suffix } => {
            value.strip_suffix(suffix).unwrap_or(&value).to_string()
        }

        Transformation::UrlNormalize { base_url: _, rules: _ } => value,
        _ => value
    };

    Ok(result)
}