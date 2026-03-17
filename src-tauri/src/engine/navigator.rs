use crate::config::paths::*;
use crate::constants::USER_AGENT;
use async_recursion::async_recursion;
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::{Duration, Instant};

// ============================================================================
// NAVIGATION CONTEXT
// ============================================================================

#[derive(Debug, Clone)]
pub struct NavigationContext {
    /// Current URL being processed
    pub current_url: String,

    /// Current value (result of last extraction)
    pub current_value: Option<String>,

    /// Response body of last fetch
    pub response_body: String,

    /// Last HTTP status code
    pub last_status: u16,

    /// Last response headers
    pub last_headers: HashMap<String, String>,

    /// Cookies string
    pub cookies: Option<String>,

    /// User-defined variables
    pub vars: HashMap<String, String>,

    /// Extracted metadata
    pub metadata: HashMap<String, String>,

    /// Navigation stack (for debugging)
    pub navigation_stack: Vec<String>,

    /// Current depth
    pub depth: usize,

    /// Maximum allowed depth
    pub max_depth: usize,

    /// Start time
    pub start_time: Instant,

    /// Timeout
    pub timeout: Duration,

    /// Source ID for auth/cookies lookup
    pub source_id: String,

    /// Named paths available for reference
    pub paths: HashMap<String, NavigationPath>,

    /// Accumulated results
    pub results: Vec<ResolvedLink>,

    /// Warnings/errors encountered
    pub warnings: Vec<String>,
}

impl NavigationContext {
    pub fn new(
        initial_url: String,
        source_id: String,
        cookies: Option<String>,
        paths: HashMap<String, NavigationPath>,
    ) -> Self {
        NavigationContext {
            current_url: initial_url,
            current_value: None,
            response_body: String::new(),
            last_status: 0,
            last_headers: HashMap::new(),
            cookies,
            vars: HashMap::new(),
            metadata: HashMap::new(),
            navigation_stack: Vec::new(),
            depth: 0,
            max_depth: 10,
            start_time: Instant::now(),
            timeout: Duration::from_secs(30),
            source_id,
            paths,
            results: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout = Duration::from_millis(timeout_ms);
        self
    }

    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Check if we've exceeded timeout
    pub fn is_timed_out(&self) -> bool {
        self.start_time.elapsed() > self.timeout
    }

    /// Check if we've exceeded max depth
    pub fn is_too_deep(&self) -> bool {
        self.depth > self.max_depth
    }

    /// Interpolate variables in a string: {var_name} -> value
    pub fn interpolate(&self, template: &str) -> String {
        let mut result = template.to_string();

        // Replace {url} with current URL
        result = result.replace("{url}", &self.current_url);

        // Replace {value} with current value
        if let Some(ref val) = self.current_value {
            result = result.replace("{value}", val);
        }

        // Replace {var_name} with variable values
        for (key, value) in &self.vars {
            result = result.replace(&format!("{{{}}}", key), value);
        }

        // Replace {meta.key} with metadata
        for (key, value) in &self.metadata {
            result = result.replace(&format!("{{meta.{}}}", key), value);
        }

        result
    }

    /// Create a child context for nested navigation
    pub fn child(&self, url: String) -> Self {
        let mut child = self.clone();
        child.current_url = url;
        child.current_value = None;
        child.response_body = String::new();
        child.depth += 1;
        child.navigation_stack.push(self.current_url.clone());
        child
    }

    /// Add a warning message
    pub fn warn(&mut self, message: impl Into<String>) {
        let msg = message.into();
        log::warn!("[Navigator] {}", msg);
        self.warnings.push(msg);
    }

    /// Add a resolved link to results
    pub fn add_result(&mut self, link: ResolvedLink) {
        self.results.push(link);
    }
}

// ============================================================================
// NAVIGATION ENGINE
// ============================================================================

pub struct Navigator {
    client: reqwest::Client,
}

impl Navigator {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Navigator { client }
    }

    /// Execute a navigation path
    #[async_recursion]
    pub async fn execute(
        &self,
        path: &NavigationPath,
        ctx: &mut NavigationContext,
    ) -> Result<Vec<ResolvedLink>, String> {
        log::debug!(
            "[Navigator] Starting path execution, depth={}, url={}",
            ctx.depth,
            ctx.current_url
        );

        // Check limits
        if ctx.is_timed_out() {
            return Err("Navigation timeout exceeded".to_string());
        }
        if ctx.is_too_deep() {
            return Err(format!("Maximum depth {} exceeded", ctx.max_depth));
        }

        // Execute steps
        for (i, step) in path.steps.iter().enumerate() {
            log::debug!("[Navigator] Executing step {}: {:?}", i, step);

            if ctx.is_timed_out() {
                ctx.warn("Timeout during step execution");
                break;
            }

            match self.execute_step(step, ctx).await {
                Ok(StepResult::Continue) => continue,
                Ok(StepResult::Return(link)) => {
                    ctx.add_result(link);
                    break;
                }
                Ok(StepResult::ReturnMultiple(links)) => {
                    for link in links {
                        ctx.add_result(link);
                    }
                    // Don't break — extract_all is a "collect" operation;
                    // subsequent steps (e.g. a second extract_all) can still run.
                }
                Ok(StepResult::Error(msg)) => {
                    // Handle error according to path config
                    if let Some(ref handlers) = path.on_error {
                        for handler in handlers {
                            match self.handle_error(handler, ctx).await {
                                Ok(true) => break, // Error handled, continue
                                Ok(false) => continue, // Try next handler
                                Err(e) => {
                                    ctx.warn(format!("Error handler failed: {}", e));
                                }
                            }
                        }
                    } else {
                        return Err(msg);
                    }
                }
                Err(e) => {
                    ctx.warn(format!("Step {} failed: {}", i, e));
                    if let Some(ref handlers) = path.on_error {
                        for handler in handlers {
                            if let Ok(true) = self.handle_error(handler, ctx).await {
                                break;
                            }
                        }
                    }
                }
            }
        }

        // If no explicit return, create result from current state
        if ctx.results.is_empty() && ctx.current_value.is_some() {
            ctx.add_result(ResolvedLink {
                url: ctx.current_value.clone().unwrap_or(ctx.current_url.clone()),
                label: None,
                host: None,
                size: ctx.metadata.get("size").cloned(),
                browser_only: false,
                browser_only_reason: None,
                metadata: ctx.metadata.clone(),
                resolution_path: ctx.navigation_stack.clone(),
            });
        }

        Ok(ctx.results.clone())
    }

    /// Execute a single step
    #[async_recursion]
    async fn execute_step(
        &self,
        step: &PathStep,
        ctx: &mut NavigationContext,
    ) -> Result<StepResult, String> {
        match step {
            PathStep::Fetch(config) => self.step_fetch(config, ctx).await,
            PathStep::Extract(config) => self.step_extract(config, ctx),
            PathStep::Transform(config) => self.step_transform(config, ctx),
            PathStep::Wait(config) => self.step_wait(config, ctx).await,
            PathStep::Return(config) => self.step_return(config, ctx),
            PathStep::ExtractAll(config) => self.step_extract_all(config, ctx).await,
            PathStep::Branch(config) => self.step_branch(config, ctx).await,
            PathStep::Loop(config) => self.step_loop(config, ctx).await,
            PathStep::Try(config) => self.step_try(config, ctx).await,
            PathStep::Use(config) => self.step_use(config, ctx).await,
            PathStep::SetVar(config) => self.step_set_var(config, ctx),
            PathStep::Log(config) => self.step_log(config, ctx),
            PathStep::DetectHost(config) => self.step_detect_host(config, ctx).await,
            PathStep::WebView(_config) => {
                // WebView steps require special handling from the frontend
                ctx.warn("WebView steps not supported in direct navigation");
                Ok(StepResult::Continue)
            }
        }
    }

    // ========================================================================
    // STEP IMPLEMENTATIONS
    // ========================================================================

    async fn step_fetch(
        &self,
        config: &FetchStep,
        ctx: &mut NavigationContext,
    ) -> Result<StepResult, String> {
        let url = config
            .url
            .as_ref()
            .map(|u| ctx.interpolate(u))
            .unwrap_or_else(|| ctx.current_url.clone());

        log::debug!("[Navigator] Fetching: {}", url);

        let mut request = match config.method.to_uppercase().as_str() {
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            _ => self.client.get(&url),
        };

        // Add headers
        for (key, value) in &config.headers {
            let interpolated = ctx.interpolate(value);
            request = request.header(key.as_str(), interpolated);
        }

        // Add cookies
        if let Some(ref cookies) = ctx.cookies {
            request = request.header("Cookie", cookies.as_str());
        }

        // Add body
        if let Some(ref body) = config.body {
            request = request.body(ctx.interpolate(body));
        }

        // Set redirect policy
        if !config.follow_redirects {
            request = request
                .try_clone()
                .ok_or("Failed to configure redirect policy")?;
        }

        // Execute request
        let response = request
            .send()
            .await
            .map_err(|e| format!("Fetch failed: {}", e))?;

        ctx.last_status = response.status().as_u16();

        // Store headers
        ctx.last_headers.clear();
        for (name, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                ctx.last_headers.insert(name.to_string(), v.to_string());
            }
        }

        // Handle redirects manually if not following
        if !config.follow_redirects && (ctx.last_status == 301 || ctx.last_status == 302) {
            if let Some(location) = ctx.last_headers.get("location") {
                ctx.current_value = Some(location.clone());
                ctx.current_url = location.clone();
                log::debug!("[Navigator] Redirect to: {}", location);
            }
        }

        // Read body
        ctx.response_body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        log::debug!(
            "[Navigator] Fetch complete, status={}, body_len={}",
            ctx.last_status,
            ctx.response_body.len()
        );

        // Store in variable if requested
        if let Some(ref var_name) = config.store_as {
            ctx.vars
                .insert(var_name.clone(), ctx.response_body.clone());
        }

        Ok(StepResult::Continue)
    }

    fn step_extract(&self, config: &ExtractStep, ctx: &mut NavigationContext) -> Result<StepResult, String> {
        let source = if config.from_url {
            &ctx.current_url
        } else {
            &ctx.response_body
        };

        let result = self.do_extraction(config, source, ctx)?;

        if let Some(value) = result {
            log::debug!("[Navigator] Extracted: {}", value);
            ctx.current_value = Some(value.clone());

            // Update URL if it looks like a URL
            if value.starts_with("http://") || value.starts_with("https://") {
                ctx.current_url = value.clone();
            }

            if let Some(ref var_name) = config.store_as {
                ctx.vars.insert(var_name.clone(), value);
            }
        } else if let Some(ref fallback) = config.fallback {
            ctx.current_value = Some(ctx.interpolate(fallback));
        } else {
            return Err("Extraction failed and no fallback provided".to_string());
        }

        Ok(StepResult::Continue)
    }

    fn do_extraction(
        &self,
        config: &ExtractStep,
        source: &str,
        ctx: &NavigationContext,
    ) -> Result<Option<String>, String> {
        // Determine pattern from various config options
        let pattern = config
            .pattern
            .as_ref()
            .or(config.selector.as_ref())
            .or(config.regex.as_ref())
            .ok_or("No extraction pattern specified")?;

        match config.method {
            PathExtractionMethod::Regex => {
                let re = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;

                if let Some(caps) = re.captures(source) {
                    let group = config.group.min(caps.len().saturating_sub(1));
                    Ok(caps.get(group).map(|m| m.as_str().to_string()))
                } else {
                    Ok(None)
                }
            }

            PathExtractionMethod::Selector => {
                let document = Html::parse_document(source);
                let selector =
                    Selector::parse(pattern).map_err(|_| format!("Invalid selector: {}", pattern))?;

                if let Some(element) = document.select(&selector).next() {
                    if let Some(ref attr) = config.attribute {
                        Ok(element.value().attr(attr).map(|s| s.to_string()))
                    } else {
                        Ok(Some(element.text().collect::<String>().trim().to_string()))
                    }
                } else {
                    Ok(None)
                }
            }

            PathExtractionMethod::Header => {
                Ok(ctx.last_headers.get(pattern).cloned())
            }

            PathExtractionMethod::JsonPath => {
                // Basic JSON path implementation
                let json: JsonValue = serde_json::from_str(source)
                    .map_err(|e| format!("Invalid JSON: {}", e))?;

                let value = self.extract_json_path(&json, pattern)?;
                Ok(value.map(|v| match v {
                    JsonValue::String(s) => s,
                    other => other.to_string(),
                }))
            }

            PathExtractionMethod::Text => {
                Ok(Some(source.trim().to_string()))
            }

            PathExtractionMethod::Attribute => {
                // For attribute method, pattern is the selector, attribute is required
                let document = Html::parse_document(source);
                let selector =
                    Selector::parse(pattern).map_err(|_| format!("Invalid selector: {}", pattern))?;

                if let Some(element) = document.select(&selector).next() {
                    let attr = config.attribute.as_deref().unwrap_or("href");
                    Ok(element.value().attr(attr).map(|s| s.to_string()))
                } else {
                    Ok(None)
                }
            }

            PathExtractionMethod::XPath => {
                // XPath not implemented yet
                Err("XPath extraction not yet implemented".to_string())
            }
        }
    }

    fn extract_json_path(&self, json: &JsonValue, path: &str) -> Result<Option<JsonValue>, String> {
        // Simple JSON path: $.field.subfield or $.array[0].field
        let parts: Vec<&str> = path
            .trim_start_matches("$.")
            .split('.')
            .collect();

        let mut current = json;

        for part in parts {
            // Handle array index
            if let Some(idx_start) = part.find('[') {
                let field = &part[..idx_start];
                let idx_str = &part[idx_start + 1..part.len() - 1];

                if !field.is_empty() {
                    current = current.get(field).ok_or(format!("Field not found: {}", field))?;
                }

                if idx_str == "*" {
                    // Return array as-is for wildcard
                    return Ok(Some(current.clone()));
                } else {
                    let idx: usize = idx_str.parse().map_err(|_| "Invalid array index")?;
                    current = current.get(idx).ok_or(format!("Array index out of bounds: {}", idx))?;
                }
            } else {
                current = current.get(part).ok_or(format!("Field not found: {}", part))?;
            }
        }

        Ok(Some(current.clone()))
    }

    fn step_transform(
        &self,
        config: &TransformStep,
        ctx: &mut NavigationContext,
    ) -> Result<StepResult, String> {
        let mut value = ctx.current_value.clone().unwrap_or_default();

        for transform in &config.transformations {
            value = self.apply_transformation(transform, &value, ctx)?;
        }

        ctx.current_value = Some(value.clone());

        // Update URL if result looks like URL
        if value.starts_with("http://") || value.starts_with("https://") {
            ctx.current_url = value;
        }

        Ok(StepResult::Continue)
    }

    fn apply_transformation(
        &self,
        transform: &PathTransformation,
        value: &str,
        ctx: &NavigationContext,
    ) -> Result<String, String> {
        match transform {
            PathTransformation::Replace { pattern, replacement } => {
                let re = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
                Ok(re.replace_all(value, replacement.as_str()).to_string())
            }

            PathTransformation::Template { template } => {
                let mut result = ctx.interpolate(template);
                result = result.replace("{value}", value);
                Ok(result)
            }

            PathTransformation::Prepend { value: prefix } => {
                Ok(format!("{}{}", ctx.interpolate(prefix), value))
            }

            PathTransformation::Append { value: suffix } => {
                Ok(format!("{}{}", value, ctx.interpolate(suffix)))
            }

            PathTransformation::Trim => {
                Ok(value.trim().to_string())
            }

            PathTransformation::UrlEncode => {
                Ok(urlencoding::encode(value).to_string())
            }

            PathTransformation::UrlDecode => {
                urlencoding::decode(value)
                    .map(|s| s.to_string())
                    .map_err(|e| format!("URL decode failed: {}", e))
            }

            PathTransformation::Base64Decode => {
                use base64::{Engine as _, engine::general_purpose::STANDARD};
                let bytes = STANDARD.decode(value)
                    .map_err(|e| format!("Base64 decode failed: {}", e))?;
                String::from_utf8(bytes)
                    .map_err(|e| format!("UTF-8 decode failed: {}", e))
            }

            PathTransformation::JsonExtract { path } => {
                let json: JsonValue = serde_json::from_str(value)
                    .map_err(|e| format!("Invalid JSON: {}", e))?;
                let extracted = self.extract_json_path(&json, path)?;
                Ok(extracted.map(|v| match v {
                    JsonValue::String(s) => s,
                    other => other.to_string(),
                }).unwrap_or_default())
            }

            PathTransformation::Join { separator } => {
                // Assume value is JSON array
                let json: JsonValue = serde_json::from_str(value)
                    .map_err(|e| format!("Invalid JSON: {}", e))?;
                if let JsonValue::Array(arr) = json {
                    let strings: Vec<String> = arr.iter().map(|v| match v {
                        JsonValue::String(s) => s.clone(),
                        other => other.to_string(),
                    }).collect();
                    Ok(strings.join(separator))
                } else {
                    Ok(value.to_string())
                }
            }

            PathTransformation::Split { separator, index } => {
                let parts: Vec<&str> = value.split(separator).collect();
                if let Some(idx) = index {
                    Ok(parts.get(*idx).map(|s| s.to_string()).unwrap_or_default())
                } else {
                    // Return as JSON array
                    Ok(serde_json::to_string(&parts).unwrap_or_default())
                }
            }
        }
    }

    async fn step_wait(&self, config: &WaitStep, ctx: &NavigationContext) -> Result<StepResult, String> {
        let ms = if let Some(ms) = config.ms {
            ms
        } else if let Some(secs) = config.seconds {
            secs * 1000
        } else if let Some(ref var) = config.var {
            let val = ctx.vars.get(var).ok_or(format!("Variable not found: {}", var))?;
            val.parse::<u64>().map_err(|_| "Invalid wait duration")?
        } else {
            0
        };

        if ms > 0 {
            log::debug!("[Navigator] Waiting {}ms", ms);
            tokio::time::sleep(Duration::from_millis(ms)).await;
        }

        Ok(StepResult::Continue)
    }

    fn step_return(&self, config: &ReturnStep, ctx: &mut NavigationContext) -> Result<StepResult, String> {
        if let Some(ref error) = config.error {
            return Ok(StepResult::Error(ctx.interpolate(error)));
        }

        let url = if config.current {
            ctx.current_value.clone().unwrap_or(ctx.current_url.clone())
        } else if let Some(ref value) = config.value {
            ctx.interpolate(value)
        } else {
            ctx.current_url.clone()
        };

        let link = ResolvedLink {
            url,
            label: ctx.metadata.get("label").cloned(),
            host: ctx.metadata.get("host").cloned(),
            size: ctx.metadata.get("size").cloned(),
            browser_only: config.browser_only,
            browser_only_reason: config.reason.clone(),
            metadata: {
                let mut meta = ctx.metadata.clone();
                for (k, v) in &config.metadata {
                    meta.insert(k.clone(), ctx.interpolate(v));
                }
                meta
            },
            resolution_path: ctx.navigation_stack.clone(),
        };

        Ok(StepResult::Return(link))
    }

    #[async_recursion]
    async fn step_extract_all(
        &self,
        config: &ExtractAllStep,
        ctx: &mut NavigationContext,
    ) -> Result<StepResult, String> {
        // Extract all data synchronously first, then drop the document
        // This is needed because Html is not Send, and we need the future to be Send for Tauri
        let extracted_items: Vec<(String, HashMap<String, String>)> = {
            let document = Html::parse_document(&ctx.response_body);
            let selector = Selector::parse(&config.selector)
                .map_err(|_| format!("Invalid selector: {}", config.selector))?;

            let mut items: Vec<(String, HashMap<String, String>)> = Vec::new();

            for element in document.select(&selector) {
                let attr_value = element
                    .value()
                    .attr(&config.attribute)
                    .map(|s| s.to_string());

                if let Some(value) = attr_value {
                    // Apply filter if specified
                    if let Some(ref filter) = config.filter {
                        let mut temp_ctx = ctx.clone();
                        temp_ctx.current_value = Some(value.clone());
                        temp_ctx.current_url = value.clone();
                        if !self.evaluate_condition(filter, &temp_ctx) {
                            continue;
                        }
                    }

                    // Extract metadata
                    let mut meta = HashMap::new();
                    for (key, extraction) in &config.extract_meta {
                        if let Some(ref static_value) = extraction.value {
                            meta.insert(key.clone(), static_value.clone());
                        } else if let Some(ref selector_str) = extraction.selector {
                            if let Ok(sel) = Selector::parse(selector_str) {
                                if let Some(el) = element.select(&sel).next() {
                                    let text = if let Some(ref attr) = extraction.attribute {
                                        el.value().attr(attr).map(|s| s.to_string())
                                    } else {
                                        Some(el.text().collect::<String>().trim().to_string())
                                    };
                                    if let Some(t) = text {
                                        meta.insert(key.clone(), t);
                                    }
                                }
                            }
                        }
                    }

                    items.push((value, meta));

                    // Check limit
                    if let Some(limit) = config.limit {
                        if items.len() >= limit {
                            break;
                        }
                    }
                }
            }
            items
        }; // document dropped here

        log::debug!("[Navigator] Extracted {} items from selector", extracted_items.len());

        // Process each extracted item
        let mut all_results: Vec<ResolvedLink> = Vec::new();

        for (value, meta) in extracted_items {
            let path = self.select_path_for_url(&value, &config.foreach, ctx)?;

            if let Some(path_steps) = path {
                // Create child context with extracted value available for processing
                // IMPORTANT: Put the extracted attribute value in BOTH:
                // - current_value: for conditions and {value} interpolation
                // - response_body: for regex/selector extraction to work on it
                // - current_url: only if it looks like a URL
                let mut child_ctx = ctx.clone();
                child_ctx.current_value = Some(value.clone());
                child_ctx.response_body = value.clone();
                child_ctx.metadata = meta.clone();
                child_ctx.depth += 1;
                child_ctx.navigation_stack.push(ctx.current_url.clone());

                // Only update current_url if it looks like a URL
                if value.starts_with("http://") || value.starts_with("https://") {
                    child_ctx.current_url = value.clone();
                }

                log::debug!("[Navigator] Processing extracted item (len={})", value.len());

                // Execute the path
                let path_obj = NavigationPath::simple(path_steps);
                match self.execute(&path_obj, &mut child_ctx).await {
                    Ok(results) => {
                        log::debug!("[Navigator] Path returned {} results", results.len());
                        all_results.extend(results);
                    }
                    Err(e) => {
                        ctx.warn(format!("Failed to process item: {}", e));
                        // Continue with other items
                    }
                }

                // Check aggregate mode
                if matches!(config.aggregate.mode, AggregateMode::FirstSuccess) && !all_results.is_empty() {
                    break;
                }
            } else {
                // No path defined - if it looks like a URL, add as result
                if value.starts_with("http://") || value.starts_with("https://") {
                    all_results.push(ResolvedLink {
                        url: value,
                        label: meta.get("label").cloned(),
                        host: None,
                        size: meta.get("size").cloned(),
                        browser_only: false,
                        browser_only_reason: None,
                        metadata: meta,
                        resolution_path: ctx.navigation_stack.clone(),
                    });
                }
            }
        }

        // Apply aggregation
        if let Some(limit) = config.aggregate.limit {
            all_results.truncate(limit);
        }

        if all_results.is_empty() {
            ctx.warn("No results from extract_all");
        }

        Ok(StepResult::ReturnMultiple(all_results))
    }

    fn select_path_for_url(
        &self,
        url: &str,
        foreach: &ForEachConfig,
        ctx: &NavigationContext,
    ) -> Result<Option<Vec<PathStep>>, String> {
        // Check match rules
        if let Some(ref rules) = foreach.match_rules {
            for rule in rules {
                let mut temp_ctx = ctx.clone();
                temp_ctx.current_url = url.to_string();

                if self.evaluate_condition(&rule.when, &temp_ctx) {
                    return Ok(Some(self.resolve_path_ref(&rule.path, ctx)?));
                }
            }
        }

        // Use default path
        if let Some(ref default) = foreach.default {
            return Ok(Some(self.resolve_path_ref(default, ctx)?));
        }

        // Use single path
        if let Some(ref path) = foreach.path {
            return Ok(Some(self.resolve_path_ref(path, ctx)?));
        }

        Ok(None)
    }

    fn resolve_path_ref(
        &self,
        path_ref: &PathOrRef,
        ctx: &NavigationContext,
    ) -> Result<Vec<PathStep>, String> {
        match path_ref {
            PathOrRef::Inline(steps) => Ok(steps.clone()),
            PathOrRef::InlineFull(path) => Ok(path.steps.clone()),
            PathOrRef::Reference(use_step) => {
                let path_name = use_step.path_ref.trim_start_matches("paths.");
                ctx.paths
                    .get(path_name)
                    .map(|p| p.steps.clone())
                    .ok_or(format!("Path not found: {}", path_name))
            }
        }
    }

    #[async_recursion]
    async fn step_branch(
        &self,
        config: &BranchStep,
        ctx: &mut NavigationContext,
    ) -> Result<StepResult, String> {
        let condition_met = self.evaluate_condition(&config.condition, ctx);

        let steps = if condition_met {
            &config.then
        } else if let Some(ref else_steps) = config.else_steps {
            else_steps
        } else {
            return Ok(StepResult::Continue);
        };

        // Execute chosen branch
        for step in steps {
            match self.execute_step(step, ctx).await? {
                StepResult::Continue => continue,
                other => return Ok(other),
            }
        }

        Ok(StepResult::Continue)
    }

    #[async_recursion]
    async fn step_loop(
        &self,
        config: &LoopStep,
        ctx: &mut NavigationContext,
    ) -> Result<StepResult, String> {
        let mut iterations = 0;

        loop {
            iterations += 1;
            if iterations > config.max {
                ctx.warn(format!("Loop max iterations ({}) reached", config.max));
                break;
            }

            // Check while condition
            if let Some(ref while_cond) = config.while_cond {
                if !self.evaluate_condition(while_cond, ctx) {
                    break;
                }
            }

            // Execute loop body
            for step in &config.steps {
                match self.execute_step(step, ctx).await? {
                    StepResult::Continue => continue,
                    StepResult::Return(link) => return Ok(StepResult::Return(link)),
                    StepResult::ReturnMultiple(links) => return Ok(StepResult::ReturnMultiple(links)),
                    StepResult::Error(e) => return Ok(StepResult::Error(e)),
                }
            }

            // Check until condition
            if let Some(ref until_cond) = config.until {
                if self.evaluate_condition(until_cond, ctx) {
                    break;
                }
            }
        }

        Ok(StepResult::Continue)
    }

    #[async_recursion]
    async fn step_try(
        &self,
        config: &TryStep,
        ctx: &mut NavigationContext,
    ) -> Result<StepResult, String> {
        for (i, attempt) in config.attempts.iter().enumerate() {
            log::debug!("[Navigator] Try attempt {}", i + 1);

            let steps = self.resolve_path_ref(attempt, ctx)?;
            let mut attempt_ctx = ctx.clone();

            let path = NavigationPath::simple(steps);
            match self.execute(&path, &mut attempt_ctx).await {
                Ok(results) if !results.is_empty() => {
                    // Success!
                    ctx.vars = attempt_ctx.vars;
                    ctx.metadata = attempt_ctx.metadata;
                    ctx.current_value = attempt_ctx.current_value;
                    ctx.current_url = attempt_ctx.current_url;
                    return Ok(StepResult::ReturnMultiple(results));
                }
                Ok(_) => {
                    ctx.warn(format!("Try attempt {} returned no results", i + 1));
                }
                Err(e) => {
                    ctx.warn(format!("Try attempt {} failed: {}", i + 1, e));
                }
            }
        }

        // All attempts failed
        if let Some(ref fallback) = config.on_all_fail {
            self.execute_step(fallback, ctx).await
        } else {
            Err("All try attempts failed".to_string())
        }
    }

    #[async_recursion]
    async fn step_use(
        &self,
        config: &UsePathStep,
        ctx: &mut NavigationContext,
    ) -> Result<StepResult, String> {
        let path_name = config.path_ref.trim_start_matches("paths.");
        let path = ctx
            .paths
            .get(path_name)
            .cloned()
            .ok_or(format!("Path not found: {}", path_name))?;

        // Apply variable overrides
        for (key, value) in &config.with {
            ctx.vars.insert(key.clone(), ctx.interpolate(value));
        }

        // Execute referenced path
        let results = self.execute(&path, ctx).await?;
        Ok(StepResult::ReturnMultiple(results))
    }

    fn step_set_var(
        &self,
        config: &SetVarStep,
        ctx: &mut NavigationContext,
    ) -> Result<StepResult, String> {
        let value = if let Some(ref extract) = config.extract {
            self.do_extraction(extract, &ctx.response_body, ctx)?
                .unwrap_or_default()
        } else {
            ctx.interpolate(&config.value)
        };

        ctx.vars.insert(config.name.clone(), value);
        Ok(StepResult::Continue)
    }

    fn step_log(&self, config: &LogStep, ctx: &NavigationContext) -> Result<StepResult, String> {
        let message = ctx.interpolate(&config.message);

        match config.level {
            LogLevel::Debug => log::debug!("[Navigator] {}", message),
            LogLevel::Info => log::info!("[Navigator] {}", message),
            LogLevel::Warn => log::warn!("[Navigator] {}", message),
            LogLevel::Error => log::error!("[Navigator] {}", message),
        }

        Ok(StepResult::Continue)
    }

    #[async_recursion]
    async fn step_detect_host(
        &self,
        config: &DetectHostStep,
        ctx: &mut NavigationContext,
    ) -> Result<StepResult, String> {
        let url = ctx.current_url.to_lowercase();

        for (host_pattern, path_ref) in &config.hosts {
            if url.contains(host_pattern) {
                ctx.metadata.insert("host".to_string(), host_pattern.clone());
                let steps = self.resolve_path_ref(path_ref, ctx)?;
                let path = NavigationPath::simple(steps);
                let results = self.execute(&path, ctx).await?;
                return Ok(StepResult::ReturnMultiple(results));
            }
        }

        // Use default if no match
        if let Some(ref default) = config.default {
            let steps = self.resolve_path_ref(default, ctx)?;
            let path = NavigationPath::simple(steps);
            let results = self.execute(&path, ctx).await?;
            return Ok(StepResult::ReturnMultiple(results));
        }

        // No match, return current URL
        Ok(StepResult::Return(ResolvedLink {
            url: ctx.current_url.clone(),
            label: None,
            host: None,
            size: None,
            browser_only: false,
            browser_only_reason: None,
            metadata: HashMap::new(),
            resolution_path: ctx.navigation_stack.clone(),
        }))
    }

    // ========================================================================
    // CONDITION EVALUATION
    // ========================================================================

    fn evaluate_condition(&self, condition: &PathCondition, ctx: &NavigationContext) -> bool {
        match condition {
            // String conditions (on current URL)
            PathCondition::Contains(s) => ctx.current_url.contains(s),
            PathCondition::StartsWith(s) => ctx.current_url.starts_with(s),
            PathCondition::EndsWith(s) => ctx.current_url.ends_with(s),
            PathCondition::Matches(pattern) => {
                Regex::new(pattern).map(|re| re.is_match(&ctx.current_url)).unwrap_or(false)
            }
            PathCondition::Equals(s) => ctx.current_url == *s,
            PathCondition::IsEmpty => ctx.current_value.as_ref().map(|v| v.is_empty()).unwrap_or(true),
            PathCondition::NotEmpty => ctx.current_value.as_ref().map(|v| !v.is_empty()).unwrap_or(false),

            // HTTP conditions
            PathCondition::Status(code) => ctx.last_status == *code,
            PathCondition::StatusRange { min, max } => ctx.last_status >= *min && ctx.last_status <= *max,
            PathCondition::StatusSuccess => ctx.last_status >= 200 && ctx.last_status < 300,
            PathCondition::StatusRedirect => ctx.last_status >= 300 && ctx.last_status < 400,
            PathCondition::StatusError => ctx.last_status >= 400,

            // Header conditions
            PathCondition::HasHeader(name) => ctx.last_headers.contains_key(&name.to_lowercase()),
            PathCondition::HeaderEquals { name, value } => {
                ctx.last_headers.get(&name.to_lowercase()).map(|v| v == value).unwrap_or(false)
            }
            PathCondition::HeaderContains { name, value } => {
                ctx.last_headers.get(&name.to_lowercase()).map(|v| v.contains(value)).unwrap_or(false)
            }
            PathCondition::ContentType(ct) => {
                ctx.last_headers.get("content-type").map(|v| v.contains(ct)).unwrap_or(false)
            }

            // Response body conditions
            PathCondition::ResponseContains(s) => ctx.response_body.contains(s),
            PathCondition::ResponseMatches(pattern) => {
                Regex::new(pattern).map(|re| re.is_match(&ctx.response_body)).unwrap_or(false)
            }
            PathCondition::SelectorExists(selector) => {
                let doc = Html::parse_document(&ctx.response_body);
                Selector::parse(selector).map(|sel| doc.select(&sel).next().is_some()).unwrap_or(false)
            }
            PathCondition::SelectorCount { selector, min, max } => {
                let doc = Html::parse_document(&ctx.response_body);
                if let Ok(sel) = Selector::parse(selector) {
                    let count = doc.select(&sel).count();
                    let min_ok = min.map(|m| count >= m).unwrap_or(true);
                    let max_ok = max.map(|m| count <= m).unwrap_or(true);
                    min_ok && max_ok
                } else {
                    false
                }
            }

            // Variable conditions
            PathCondition::VarEquals { var, value } => {
                ctx.vars.get(var).map(|v| v == value).unwrap_or(false)
            }
            PathCondition::VarContains { var, value } => {
                ctx.vars.get(var).map(|v| v.contains(value)).unwrap_or(false)
            }
            PathCondition::VarExists(var) => ctx.vars.contains_key(var),
            PathCondition::VarMatches { var, pattern } => {
                ctx.vars.get(var).and_then(|v| {
                    Regex::new(pattern).ok().map(|re| re.is_match(v))
                }).unwrap_or(false)
            }

            // URL-specific conditions
            PathCondition::UrlContains(s) => ctx.current_url.contains(s),
            PathCondition::UrlMatches(pattern) => {
                Regex::new(pattern).map(|re| re.is_match(&ctx.current_url)).unwrap_or(false)
            }
            PathCondition::HostEquals(host) => {
                url::Url::parse(&ctx.current_url)
                    .ok()
                    .and_then(|u| u.host_str().map(|h| h == host))
                    .unwrap_or(false)
            }
            PathCondition::HostContains(s) => {
                url::Url::parse(&ctx.current_url)
                    .ok()
                    .and_then(|u| u.host_str().map(|h| h.contains(s)))
                    .unwrap_or(false)
            }

            // Logical operators
            PathCondition::And(conditions) => conditions.iter().all(|c| self.evaluate_condition(c, ctx)),
            PathCondition::Or(conditions) => conditions.iter().any(|c| self.evaluate_condition(c, ctx)),
            PathCondition::Not(condition) => !self.evaluate_condition(condition, ctx),

            // Always/Never
            PathCondition::Always => true,
            PathCondition::Never => false,
        }
    }

    // ========================================================================
    // ERROR HANDLING
    // ========================================================================

    async fn handle_error(
        &self,
        handler: &ErrorHandler,
        ctx: &mut NavigationContext,
    ) -> Result<bool, String> {
        match handler {
            ErrorHandler::Use(use_step) => {
                self.step_use(use_step, ctx).await?;
                Ok(true)
            }
            ErrorHandler::Retry { max_attempts, delay_ms: _ } => {
                // Retry logic would need to be implemented at a higher level
                ctx.warn(format!("Retry not yet implemented (would retry {} times)", max_attempts));
                Ok(false)
            }
            ErrorHandler::ReturnOriginal => {
                ctx.add_result(ResolvedLink {
                    url: ctx.navigation_stack.first().cloned().unwrap_or(ctx.current_url.clone()),
                    label: None,
                    host: None,
                    size: None,
                    browser_only: false,
                    browser_only_reason: None,
                    metadata: HashMap::new(),
                    resolution_path: vec![],
                });
                Ok(true)
            }
            ErrorHandler::ReturnError { message } => {
                Err(ctx.interpolate(message))
            }
            ErrorHandler::Continue => {
                Ok(true)
            }
        }
    }
}

// ============================================================================
// STEP RESULT
// ============================================================================

enum StepResult {
    Continue,
    Return(ResolvedLink),
    ReturnMultiple(Vec<ResolvedLink>),
    Error(String),
}

impl Default for Navigator {
    fn default() -> Self {
        Self::new()
    }
}
