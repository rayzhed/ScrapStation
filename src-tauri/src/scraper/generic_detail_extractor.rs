use scraper::{Html, Selector};
use crate::constants::USER_AGENT;
use crate::types::detail_section::*;
use reqwest;

pub fn extract_sections_from_html(
    html_content: &str,
    sections: &[DetailSection],
) -> Result<Vec<ExtractedSection>, String> {
    let document = Html::parse_document(html_content);
    extract_from_document(&document, sections)
}

pub async fn extract_sections_with_cookies(
    url: &str,
    sections: &[DetailSection],
    cookies: Option<&str>,
) -> Result<Vec<ExtractedSection>, String> {
    let html_content = fetch_html_with_cookies(url, cookies).await?;
    let document = Html::parse_document(&html_content);
    extract_from_document(&document, sections)
}

fn extract_from_document(
    document: &Html,
    sections: &[DetailSection],
) -> Result<Vec<ExtractedSection>, String> {

    let mut extracted_sections = Vec::new();

    for section in sections {
        match section {
            DetailSection::Hero { order, config } => {
                if let Ok(section) = extract_hero(document, *order, config) {
                    extracted_sections.push(section);
                }
            }
            DetailSection::Video { order, title, icon, config } => {
                if let Ok(Some(section)) = extract_video(document, *order, title, icon, config) {
                    extracted_sections.push(section);
                }
            }
            DetailSection::TextContent { order, title, icon, config } => {
                if let Ok(Some(section)) = extract_text_content(document, *order, title, icon, config) {
                    extracted_sections.push(section);
                }
            }
            DetailSection::MetadataGrid { order, title, icon, config } => {
                if let Ok(section) = extract_metadata_grid(document, *order, title, icon, config) {
                    extracted_sections.push(section);
                }
            }
            DetailSection::NumberedSteps { order, title, icon, config } => {
                if let Ok(Some(section)) = extract_numbered_steps(document, *order, title, icon, config) {
                    extracted_sections.push(section);
                }
            }
            DetailSection::AlertBox { order, title, icon, style, config } => {
                if let Ok(Some(section)) = extract_alert_box(document, *order, title, icon, style, config) {
                    extracted_sections.push(section);
                }
            }
            DetailSection::DownloadButtons { order, title, icon, config } => {
                if let Ok(section) = extract_download_buttons(document, *order, title, icon, config) {
                    extracted_sections.push(section);
                }
            }
            DetailSection::Dynamic { order, title, icon, style, config } => {
                if let Ok(section) = extract_dynamic_section(document, *order, title, icon, style.as_deref(), config) {
                    extracted_sections.push(section);
                }
            }
        }
    }

    extracted_sections.sort_by_key(|s| s.order);

    Ok(extracted_sections)
}

async fn fetch_html_with_cookies(url: &str, cookies: Option<&str>) -> Result<String, String> {
    log::debug!(
        "[FetchHTML] Fetching URL: {}, has cookies: {}",
        url,
        cookies.is_some()
    );

    let referer = url::Url::parse(url)
        .map(|u| format!("{}://{}/", u.scheme(), u.host_str().unwrap_or("")))
        .unwrap_or_else(|_| url.to_string());

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut request = client
        .get(url)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Referer", &referer);

    if let Some(cookie_str) = cookies {
        log::debug!("[FetchHTML] Adding cookies to request");
        request = request.header("Cookie", cookie_str);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to fetch page: {}", e))?;

    let status = response.status();
    log::debug!("[FetchHTML] Response status: {}", status);

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    log::debug!("[FetchHTML] Response body length: {} chars", body.len());

    // Check if we got a Cloudflare challenge or login page
    if body.contains("challenge-running") || body.contains("cf-browser-verification") {
        log::warn!("[FetchHTML] Cloudflare challenge detected! WebView auth may be required.");
    }

    if body.contains("Вы не авторизованы") || body.contains("login") && body.len() < 5000 {
        log::warn!("[FetchHTML] Page may require authentication - got possible login page");
    }

    Ok(body)
}

fn extract_hero(
    document: &Html,
    order: u32,
    config: &HeroConfig,
) -> Result<ExtractedSection, String> {

    let background_image = extract_by_selector(document, &config.background_image).ok();
    let title = extract_by_selector(document, &config.title)?;
    let subtitle = config.subtitle.as_ref()
        .and_then(|s| extract_by_selector(document, s).ok());

    let mut badges = Vec::new();
    for badge_config in &config.badges {
        if let Ok(value) = extract_badge_value(document, badge_config) {
            badges.push(serde_json::json!({
                "label": badge_config.label,
                "icon": badge_config.icon,
                "value": value,
                "style": badge_config.style,
            }));
        }
    }

    Ok(ExtractedSection {
        section_type: "hero".to_string(),
        order,
        title: None,
        icon: None,
        style: None,
        data: serde_json::json!({
            "background_image": background_image,
            "title": title,
            "subtitle": subtitle,
            "badges": badges,
        }),
    })
}

fn extract_badge_value(
    document: &Html,
    config: &BadgeConfig,
) -> Result<String, String> {

    let selector = Selector::parse(&config.selector)
        .map_err(|e| format!("Invalid selector: {}", e))?;

    for element in document.select(&selector) {
        let text = element.text().collect::<String>();

        if let Some(contains) = &config.contains {
            if !text.contains(contains) {
                continue;
            }
        }

        if let Some(extract_after) = &config.extract_after {
            if let Some(value) = text.split(extract_after).nth(1) {
                let trimmed = value.trim().to_string();
                return Ok(trimmed);
            }
        }

        if config.extract_number == Some(true) {
            if let Some(num_str) = text.split(':').nth(1) {
                if let Ok(num) = num_str.trim().parse::<u32>() {
                    let mut result = num.to_string();
                    if let Some(suffix) = &config.suffix {
                        result.push_str(suffix);
                    }
                    return Ok(result);
                }
            }
        }

        return Ok(text.trim().to_string());
    }

    Err("Badge value not found".to_string())
}

fn extract_by_selector(
    document: &Html,
    config: &SelectorConfig,
) -> Result<String, String> {
    let selector = Selector::parse(&config.selector)
        .map_err(|e| format!("Invalid selector: {}", e))?;

    let element = document.select(&selector).next()
        .ok_or("Element not found")?;

    if let Some(attr) = &config.attribute {
        Ok(element.value().attr(attr).unwrap_or("").to_string())
    } else {
        Ok(element.text().collect::<String>().trim().to_string())
    }
}

fn extract_video(
    document: &Html,
    order: u32,
    title: &str,
    icon: &str,
    config: &VideoConfig,
) -> Result<Option<ExtractedSection>, String> {

    let selector = Selector::parse(&config.selector)
        .map_err(|e| format!("Invalid selector: {}", e))?;

    if let Some(element) = document.select(&selector).next() {
        let mut video_url = element.value()
            .attr(&config.attribute)
            .unwrap_or("")
            .to_string();

        if config.transform == Some("youtube_embed".to_string())
            && (video_url.contains("youtube-nocookie.com/embed/") || video_url.contains("youtube.com/embed/"))
        {
            video_url = video_url.replace("youtube-nocookie.com", "youtube.com");
        }

        if !video_url.is_empty() {
            return Ok(Some(ExtractedSection {
                section_type: "video".to_string(),
                order,
                title: Some(title.to_string()),
                icon: Some(icon.to_string()),
                style: None,
                data: serde_json::json!({ "url": video_url }),
            }));
        }
    }

    Ok(None)
}

fn extract_text_content(
    document: &Html,
    order: u32,
    title: &str,
    icon: &str,
    config: &TextContentConfig,
) -> Result<Option<ExtractedSection>, String> {

    let selector = Selector::parse(&config.selector)
        .map_err(|e| format!("Invalid selector: {}", e))?;

    if let Some(element) = document.select(&selector).next() {
        let mut content = if config.extract == "html" {
            element.html()
        } else {
            element.text().collect::<String>()
        };

        content = content.trim().to_string();

        // Clean the content
        content = content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        if let Some(max_length) = config.max_length {
            if content.chars().count() > max_length {
                content = content.chars().take(max_length).collect::<String>();
                content.push_str("...");
            }
        }

        if !content.is_empty() {
            return Ok(Some(ExtractedSection {
                section_type: "text_content".to_string(),
                order,
                title: Some(title.to_string()),
                icon: Some(icon.to_string()),
                style: None,
                data: serde_json::json!({ "content": content }),
            }));
        }
    }

    Ok(None)
}

fn extract_metadata_grid(
    document: &Html,
    order: u32,
    title: &str,
    icon: &str,
    config: &MetadataGridConfig,
) -> Result<ExtractedSection, String> {

    let mut items = Vec::new();

    for item_config in &config.items {
        let selector = Selector::parse(&item_config.selector)
            .map_err(|e| format!("Invalid selector: {}", e))?;

        for element in document.select(&selector) {
            let text = element.text().collect::<String>();

            if let Some(contains) = &item_config.contains {
                if !text.contains(contains) {
                    continue;
                }
            }

            let mut value = if let Some(attr) = &item_config.attribute {
                element.value().attr(attr).unwrap_or("").to_string()
            } else {
                text.clone()
            };

            if let Some(extract_after) = &item_config.extract_after {
                if let Some(extracted) = value.split(extract_after).nth(1) {
                    value = extracted.trim().to_string();
                }
            }

            if !value.is_empty() {
                items.push(serde_json::json!({
                    "label": item_config.label,
                    "icon": item_config.icon,
                    "value": value,
                    "render_as": item_config.render_as,
                    "style": item_config.style,
                }));
                break;
            }
        }
    }

    Ok(ExtractedSection {
        section_type: "metadata_grid".to_string(),
        order,
        title: Some(title.to_string()),
        icon: Some(icon.to_string()),
        style: None,
        data: serde_json::json!({ "items": items }),
    })
}

fn extract_numbered_steps(
    document: &Html,
    order: u32,
    title: &str,
    icon: &str,
    config: &NumberedStepsConfig,
) -> Result<Option<ExtractedSection>, String> {

    let selector = Selector::parse(&config.selector)
        .map_err(|e| format!("Invalid selector: {}", e))?;

    if let Some(element) = document.select(&selector).next() {
        let text = element.text().collect::<String>();

        let mut in_section = false;
        let mut steps = Vec::new();

        for line in text.lines() {
            let line = line.trim();

            if let Some(start_after) = &config.start_after {
                if line.contains(start_after) {
                    in_section = true;
                    continue;
                }
            } else {
                in_section = true;
            }

            if let Some(end_before) = &config.end_before {
                if line.contains(end_before) {
                    break;
                }
            }

            if in_section && !line.is_empty() {
                if let Some(first_char) = line.chars().next() {
                    if first_char.is_numeric() {
                        if let Some((num_str, instruction)) = line.split_once('.') {
                            if let Ok(step_number) = num_str.trim().parse::<usize>() {
                                steps.push(serde_json::json!({
                                    "step_number": step_number,
                                    "instruction": instruction.trim(),
                                }));
                            }
                        }
                    }
                }
            }
        }

        if !steps.is_empty() {
            return Ok(Some(ExtractedSection {
                section_type: "numbered_steps".to_string(),
                order,
                title: Some(title.to_string()),
                icon: Some(icon.to_string()),
                style: None,
                data: serde_json::json!({ "steps": steps }),
            }));
        }
    }

    Ok(None)
}

fn extract_alert_box(
    document: &Html,
    order: u32,
    title: &str,
    icon: &str,
    style: &str,
    config: &AlertBoxConfig,
) -> Result<Option<ExtractedSection>, String> {

    // Static items defined in YAML — skip DOM scraping entirely
    if let Some(items) = &config.items {
        if !items.is_empty() {
            return Ok(Some(ExtractedSection {
                section_type: "alert_box".to_string(),
                order,
                title: Some(title.to_string()),
                icon: Some(icon.to_string()),
                style: Some(style.to_string()),
                data: serde_json::json!({
                    "style": style,
                    "items": items,
                }),
            }));
        }
    }

    // DOM scraping path
    let selector_str = config.selector.as_deref()
        .ok_or("alert_box: 'selector' is required when 'items' is not provided")?;
    let items_selector_str = config.items_selector.as_deref()
        .ok_or("alert_box: 'items_selector' is required when 'items' is not provided")?;

    let selector = Selector::parse(selector_str)
        .map_err(|e| format!("Invalid selector: {}", e))?;

    for element in document.select(&selector) {
        if let Some(parent_contains) = &config.parent_contains {
            let parent_text = element.text().collect::<String>();
            if !parent_text.contains(parent_contains) {
                continue;
            }
        }

        let items_selector = Selector::parse(items_selector_str)
            .map_err(|e| format!("Invalid items selector: {}", e))?;

        let items: Vec<String> = element
            .select(&items_selector)
            .map(|item| item.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if !items.is_empty() {
            return Ok(Some(ExtractedSection {
                section_type: "alert_box".to_string(),
                order,
                title: Some(title.to_string()),
                icon: Some(icon.to_string()),
                style: Some(style.to_string()),
                data: serde_json::json!({
                    "style": style,
                    "items": items,
                }),
            }));
        }
    }

    Ok(None)
}

fn extract_download_buttons(
    document: &Html,
    order: u32,
    title: &str,
    icon: &str,
    config: &DownloadButtonsConfig,
) -> Result<ExtractedSection, String> {
    let mut buttons = Vec::new();

    log::debug!(
        "[DownloadButtons] Extracting buttons with {} selector configs",
        config.buttons.len()
    );

    for button_config in &config.buttons {
        let selector = match Selector::parse(&button_config.selector) {
            Ok(s) => s,
            Err(e) => {
                log::warn!(
                    "[DownloadButtons] Invalid selector '{}': {:?}",
                    button_config.selector,
                    e
                );
                continue;
            }
        };

        let mut found_count = 0;
        for element in document.select(&selector) {
            let url = element.value().attr("href").unwrap_or("").to_string();
            let elem_text = element.text().collect::<String>();
            let elem_text = elem_text.trim();

            // Skip elements that don't contain the required text
            if let Some(required) = &button_config.contains_text {
                if !elem_text.contains(required.as_str()) {
                    continue;
                }
            }

            if !url.is_empty() {
                found_count += 1;
                let label = if button_config.use_text_as_label == Some(true) {
                    if elem_text.is_empty() {
                        button_config.label.clone()
                    } else {
                        elem_text.to_string()
                    }
                } else {
                    button_config.label.clone()
                };

                // Use smart_download action if enabled, otherwise use configured action
                let action = if button_config.smart_download == Some(true) {
                    "smart_download".to_string()
                } else {
                    button_config.action.clone()
                };

                log::debug!(
                    "[DownloadButtons] Found button: label='{}', url='{}'",
                    label,
                    url
                );

                buttons.push(serde_json::json!({
                    "label": label,
                    "url": url,
                    "icon": button_config.icon,
                    "style": button_config.style,
                    "action": action,
                    "resolve_link": button_config.resolve_link,
                    "smart_download": button_config.smart_download,
                    "resolver": button_config.resolver,
                    "supported": button_config.supported.unwrap_or(true),
                    "warning": button_config.warning,
                }));
            }
        }

        if found_count == 0 {
            log::debug!(
                "[DownloadButtons] No elements found for selector '{}'",
                button_config.selector
            );
        }
    }

    if buttons.is_empty() {
        log::warn!(
            "[DownloadButtons] No download buttons found! Selectors tried: {:?}",
            config.buttons.iter().map(|b| &b.selector).collect::<Vec<_>>()
        );
    } else {
        log::info!(
            "[DownloadButtons] Successfully extracted {} download buttons",
            buttons.len()
        );
    }

    Ok(ExtractedSection {
        section_type: "download_buttons".to_string(),
        order,
        title: Some(title.to_string()),
        icon: Some(icon.to_string()),
        style: None,
        data: serde_json::json!({ "buttons": buttons }),
    })
}

fn extract_dynamic_section(
    document: &Html,
    order: u32,
    title: &str,
    icon: &str,
    style: Option<&str>,
    config: &DynamicSectionConfig,
) -> Result<ExtractedSection, String> {
    let mut data = serde_json::Map::new();

    for (field_name, field_config) in &config.fields {
        match extract_dynamic_field(document, field_config) {
            Ok(value) => {
                data.insert(field_name.clone(), value);
            }
            Err(_) => {
                if let Some(default) = &field_config.default {
                    data.insert(field_name.clone(), serde_json::Value::String(default.clone()));
                }
            }
        }
    }

    data.insert("_renderer".to_string(), serde_json::Value::String(config.renderer.clone()));
    if let Some(style_config) = &config.style {
        let mut style_obj = serde_json::Map::new();
        if let Some(variant) = &style_config.variant {
            style_obj.insert("variant".to_string(), serde_json::Value::String(variant.clone()));
        }
        if let Some(columns) = style_config.columns {
            style_obj.insert("columns".to_string(), serde_json::Value::Number(columns.into()));
        }
        if let Some(compact) = style_config.compact {
            style_obj.insert("compact".to_string(), serde_json::Value::Bool(compact));
        }
        data.insert("_style".to_string(), serde_json::Value::Object(style_obj));
    }

    Ok(ExtractedSection {
        section_type: "dynamic".to_string(),
        order,
        title: Some(title.to_string()),
        icon: Some(icon.to_string()),
        style: style.map(|s| s.to_string()),
        data: serde_json::Value::Object(data),
    })
}

fn extract_dynamic_field(
    document: &Html,
    config: &FieldExtraction,
) -> Result<serde_json::Value, String> {
    let selector = Selector::parse(&config.selector)
        .map_err(|e| format!("Invalid selector '{}': {:?}", config.selector, e))?;

    if config.multiple {
        // Extract multiple values
        let values: Vec<String> = document.select(&selector)
            .filter_map(|el| extract_element_value(&el, config))
            .collect();

        if values.is_empty() {
            return Err("No elements found".to_string());
        }

        Ok(serde_json::Value::Array(
            values.into_iter().map(serde_json::Value::String).collect()
        ))
    } else {
        // Extract single value
        let element = document.select(&selector).next()
            .ok_or("Element not found")?;

        let value = extract_element_value(&element, config)
            .ok_or("Failed to extract value")?;

        Ok(serde_json::Value::String(value))
    }
}

fn extract_element_value(
    element: &scraper::ElementRef,
    config: &FieldExtraction,
) -> Option<String> {
    let raw_value = match config.extract.as_str() {
        "html" => element.html(),
        "attribute" => {
            let attr = config.attribute.as_deref().unwrap_or("href");
            element.value().attr(attr)?.to_string()
        }
        _ => element.text().collect::<String>(), // "text" is default
    };

    let mut value = raw_value.trim().to_string();

    // Apply regex pattern if specified
    if let Some(pattern) = &config.pattern {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(captures) = re.captures(&value) {
                value = captures.get(1)
                    .or_else(|| captures.get(0))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or(value);
            }
        }
    }

    // Apply transform if specified
    if let Some(transform) = &config.transform {
        value = apply_transform(&value, transform);
    }

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn apply_transform(value: &str, transform: &str) -> String {
    match transform {
        "trim" => value.trim().to_string(),
        "lowercase" => value.to_lowercase(),
        "uppercase" => value.to_uppercase(),
        "strip_html" => {
            // Simple HTML stripping
            let re = regex::Regex::new(r"<[^>]*>").unwrap();
            re.replace_all(value, "").trim().to_string()
        }
        _ => value.to_string(),
    }
}