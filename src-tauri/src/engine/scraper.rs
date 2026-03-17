use crate::config::{SourceConfig, SourceType};
use crate::engine::{html_parser, json_parser, extractor, transformer, tags, link_resolver};
use crate::utils::HttpClient;
use std::collections::HashMap;

pub struct UniversalScraper;

impl UniversalScraper {
    pub async fn scrape_with_client(
        config: &SourceConfig,
        content: &str,
        is_search: bool,
        http_client: Option<&HttpClient>,
    ) -> Result<Vec<HashMap<String, String>>, String> {
        let raw_items = match config.source_type {
            SourceType::HtmlScraper => {
                if let Some(selectors) = &config.selectors {
                    html_parser::parse(content, selectors, &config.attributes, is_search)?
                } else {
                    return Err("HTML scraper requires 'selectors' config".to_string());
                }
            }
            SourceType::JsonApi | SourceType::GraphqlApi => {
                if let Some(json_paths) = &config.json_paths {
                    json_parser::parse(content, json_paths)?
                } else {
                    return Err("JSON API requires 'json_paths' config".to_string());
                }
            }
            _ => {
                return Err(format!("Source type {:?} not yet implemented", config.source_type));
            }
        };

        let transformed_items = raw_items
            .into_iter()
            .map(|item| transformer::apply_transformations(item, config))
            .collect::<Result<Vec<_>, _>>()?;

        let enriched_items = transformed_items
            .into_iter()
            .map(|item| extractor::extract_metadata(item, config))
            .collect::<Result<Vec<_>, _>>()?;

        let final_items = enriched_items
            .into_iter()
            .map(|item| extractor::apply_custom_fields(item, config))
            .collect::<Result<Vec<_>, _>>()?;

        let items_with_tags = final_items
            .into_iter()
            .map(|mut item| {
                let extracted_tags = tags::extract_tags(&item, config);

                if !extracted_tags.is_empty() {
                    if let Ok(tags_json) = serde_json::to_string(&extracted_tags) {
                        item.insert("_tags".to_string(), tags_json);
                    }
                }

                item
            })
            .collect::<Vec<_>>();

        if let (Some(link_config), Some(client)) = (&config.link_resolution, http_client) {
            link_resolver::LinkResolverEngine::resolve_links(
                items_with_tags,
                link_config,
                client,
            ).await
        } else {
            Ok(items_with_tags)
        }
    }
}
