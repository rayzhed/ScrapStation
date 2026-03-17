use crate::config::{GameCard, SourceConfig, LinkResolutionConfig};
use crate::utils::{create_client, get_or_create_rate_limiter, HttpClient};
use std::collections::HashMap;

pub async fn apply_rate_limit(source_id: &str, config: &SourceConfig) {
    if let Some(rate_config) = &config.rate_limit {
        if rate_config.enabled {
            let limiter = get_or_create_rate_limiter(
                source_id,
                rate_config.window_seconds,
                rate_config.min_delay_ms,
                rate_config.burst_threshold,
            )
            .await;
            limiter.wait().await;
        }
    }
}

pub async fn resolve_link_on_demand(
    url: &str,
    link_config: &LinkResolutionConfig,
    cookies: Option<&str>,
) -> Result<String, String> {
    log::debug!("[ResolveOnDemand] Input URL: {}", url);
    log::debug!("[ResolveOnDemand] Resolvers in config: {:?}", link_config.resolvers.keys().collect::<Vec<_>>());

    let client = create_client()?;
    let http_client = HttpClient::new(client);

    let mut on_demand_config = link_config.clone();
    on_demand_config.enabled = true;

    // Add cookies to resolver headers if provided
    if let Some(cookie_str) = cookies {
        log::debug!("[ResolveOnDemand] Adding cookies to fetch headers");
        for resolver in on_demand_config.resolvers.values_mut() {
            for step in &mut resolver.steps {
                if let crate::config::ResolutionStep::Fetch { headers, .. } = step {
                    let h = headers.get_or_insert_with(HashMap::new);
                    h.insert("Cookie".to_string(), cookie_str.to_string());
                }
            }
        }
    }

    let mut item = HashMap::new();
    item.insert("link".to_string(), url.to_string());

    log::debug!("[ResolveOnDemand] Calling LinkResolverEngine::resolve_links");

    let items = crate::engine::link_resolver::LinkResolverEngine::resolve_links(
        vec![item],
        &on_demand_config,
        &http_client,
    )
    .await?;

    if let Some(resolved_item) = items.first() {
        log::debug!("[ResolveOnDemand] Resolved item keys: {:?}", resolved_item.keys().collect::<Vec<_>>());
        for (k, v) in resolved_item {
            log::debug!("[ResolveOnDemand]   {} = {}", k, v);
        }
        let resolved = resolved_item
            .values()
            .find(|v| *v != url)
            .cloned()
            .unwrap_or_else(|| {
                log::warn!("[ResolveOnDemand] No different value found, returning original URL");
                url.to_string()
            });
        log::info!("[ResolveOnDemand] Final resolved URL: {}", resolved);
        Ok(resolved)
    } else {
        log::warn!("[ResolveOnDemand] No items returned from resolver");
        Ok(url.to_string())
    }
}

pub fn build_url(config: &SourceConfig, page: u32) -> Result<String, String> {
    let url = if page == 1 {
        format!(
            "{}{}",
            config.base_url,
            config.urls.main.as_ref().ok_or("No main URL in config")?
        )
    } else {
        let page_template = config.urls.page.as_ref().ok_or("No page URL template in config")?;
        format!(
            "{}{}",
            config.base_url,
            page_template.replace("{page}", &page.to_string())
        )
    };

    Ok(url)
}

pub fn convert_to_game_cards(
    raw_items: Vec<HashMap<String, String>>,
    config: &SourceConfig,
) -> Result<Vec<GameCard>, String> {
    raw_items
        .into_iter()
        .map(|item| map_to_game_card(item, config))
        .collect()
}

fn map_to_game_card(item: HashMap<String, String>, config: &SourceConfig) -> Result<GameCard, String> {
    let mapping = config.field_mapping.as_ref();
    let defaults = config.default_values.as_ref();

    let get_field = |field_name: &str, default_key: Option<&str>| -> String {
        item.get(field_name)
            .cloned()
            .or_else(|| default_key.and_then(|k| defaults.and_then(|d| d.get(k).cloned())))
            .unwrap_or_default()
    };

    let tags = if let Some(tags_json) = item.get("_tags") {
        serde_json::from_str(tags_json).unwrap_or_default()
    } else {
        Vec::new()
    };

    let title = get_field(
        mapping.and_then(|m| m.title.as_deref()).unwrap_or("title"),
        None,
    );
    let cover_url = get_field(
        mapping.and_then(|m| m.cover_url.as_deref()).unwrap_or("cover"),
        None,
    );
    let game_url = get_field(
        mapping.and_then(|m| m.game_url.as_deref()).unwrap_or("link"),
        None,
    );

    let mut extra = std::collections::HashMap::new();

    let to_json_value = |value: &str| -> serde_json::Value {
        if value == "true" || value == "false" {
            serde_json::Value::Bool(value == "true")
        } else if let Ok(num) = value.parse::<i64>() {
            serde_json::Value::Number(num.into())
        } else if let Ok(num) = value.parse::<f64>() {
            serde_json::json!(num)
        } else {
            serde_json::Value::String(value.to_string())
        }
    };

    for (key, value) in &item {
        if key.starts_with('_') || key == "title" || key == "cover" || key == "link" {
            continue;
        }
        extra.insert(key.clone(), to_json_value(value));
    }

    if let Some(m) = mapping {
        macro_rules! apply_mapping {
            ($field:expr, $name:expr) => {
                if let Some(source_field) = $field {
                    if let Some(val) = item.get(source_field.as_str()) {
                        extra.insert($name.to_string(), to_json_value(val));
                    }
                }
            };
        }

        apply_mapping!(&m.description, "description");
        apply_mapping!(&m.genre, "genre");
        apply_mapping!(&m.author, "author");

        for (target_name, source_field) in &m.custom {
            if let Some(val) = item.get(source_field.as_str()) {
                extra.insert(target_name.clone(), to_json_value(val));
            }
        }
    }

    if let Some(defaults) = defaults {
        for (key, val) in defaults {
            if !extra.contains_key(key) {
                extra.insert(key.clone(), serde_json::Value::String(val.clone()));
            }
        }
    }

    Ok(GameCard {
        title,
        cover_url,
        game_url,
        tags,
        extra,
    })
}
