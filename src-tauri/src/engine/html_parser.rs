use crate::config::FieldSelector;
use scraper::{Html, Selector, ElementRef};
use std::collections::HashMap;

pub fn parse(
    html: &str,
    selectors_config: &HashMap<String, crate::config::PageSelectors>,
    attributes_config: &Option<HashMap<String, Vec<String>>>,
    is_search: bool,
) -> Result<Vec<HashMap<String, String>>, String> {
    let document = Html::parse_document(html);

    // Select appropriate configuration
    let page_key = if is_search { "search_page" } else { "main_page" };

    let page_selectors = selectors_config.get(page_key)
        .or_else(|| selectors_config.get("main_page"))
        .ok_or("No selectors found for this page type")?;

    let container_selector = Selector::parse(&page_selectors.container)
        .map_err(|e| format!("Invalid container selector: {:?}", e))?;

    let containers: Vec<_> = document.select(&container_selector).collect();
    log::debug!("[HtmlParser] selector='{}' → {} containers found", page_selectors.container, containers.len());

    if containers.is_empty() {
        // Find <body> start and dump content from there to identify real selectors
        let full_html = document.root_element().html();
        let body_start = full_html.find("<body").unwrap_or(0);
        let snippet: String = full_html[body_start..].chars().take(2000).collect();
        log::warn!("[HtmlParser] No containers matched '{}'. Body snippet:\n{}", page_selectors.container, snippet);

        // Also log all unique class names found in the first 100 div/article/section elements
        use scraper::Selector;
        if let Ok(sel) = Selector::parse("div[class], article[class], section[class], ul[class]") {
            let classes: Vec<String> = document.select(&sel)
                .take(30)
                .filter_map(|el| el.value().attr("class"))
                .map(|c| c.split_whitespace().next().unwrap_or("").to_string())
                .filter(|c| !c.is_empty())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            log::warn!("[HtmlParser] Top-level classes found: {:?}", classes);
        }
    }

    let mut items = Vec::new();

    for (index, container) in containers.iter().enumerate() {
        // Log the inner HTML of the first container to diagnose field selectors
        if index == 0 {
            let inner: String = container.inner_html().chars().take(1500).collect();
            log::debug!("[HtmlParser] First container inner HTML:\n{}", inner);
        }

        let mut item: HashMap<String, String> = HashMap::new();

        for (field_name, field_selector) in &page_selectors.fields {
            if let Some(value) = extract_field(container, field_selector, attributes_config) {
                item.insert(field_name.clone(), value);
            }
        }

        if !item.is_empty() {
            log::debug!("[HtmlParser] item #{}: {} fields", index + 1, item.len());
            items.push(item);
        }
    }

    Ok(items)
}

fn extract_field(
    container: &ElementRef,
    field_selector: &FieldSelector,
    attributes_config: &Option<HashMap<String, Vec<String>>>,
) -> Option<String> {
    match field_selector {
        FieldSelector::Simple(selector_str) => {
            let selector = Selector::parse(selector_str).ok()?;
            let element = container.select(&selector).next()?;
            Some(element.text().collect::<String>().trim().to_string())
        }

        FieldSelector::WithAttribute { selector, attribute, multiple } => {
            // Special "self" selector: extract from the container element itself
            if selector == "self" || selector.is_empty() {
                return if let Some(attr_name) = attribute {
                    container.value().attr(attr_name.as_str()).map(|s| s.to_string())
                } else {
                    Some(container.text().collect::<String>().trim().to_string())
                };
            }

            let sel = Selector::parse(selector).ok()?;

            if *multiple {
                let values: Vec<String> = container.select(&sel)
                    .filter_map(|elem| {
                        if let Some(attr) = attribute {
                            elem.value().attr(attr).map(|s| s.to_string())
                        } else {
                            Some(elem.text().collect::<String>().trim().to_string())
                        }
                    })
                    .collect();

                if values.is_empty() {
                    None
                } else {
                    Some(values.join(", "))
                }
            } else {
                let element = container.select(&sel).next()?;

                if let Some(attr_name) = attribute {
                    // Try fallback attributes
                    let default_attrs = vec![attr_name.clone()];
                    let attrs = attributes_config.as_ref()
                        .and_then(|a| a.get(attr_name))
                        .unwrap_or(&default_attrs);

                    attrs.iter()
                        .find_map(|attr: &String| element.value().attr(attr))
                        .map(|s: &str| s.to_string())
                } else {
                    Some(element.text().collect::<String>().trim().to_string())
                }
            }
        }

        FieldSelector::Static { value } => {
            Some(value.clone())
        }
    }
}