use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

// ============================================================================
// PATH SYSTEM - Flexible tree-based link resolution
// ============================================================================

/// A navigation path - sequence of steps to resolve a link
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NavigationPath {
    /// Named identifier for this path (for reuse)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// The steps to execute
    pub steps: Vec<PathStep>,

    /// What to do on error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_error: Option<Vec<ErrorHandler>>,

    /// Maximum depth for nested navigation
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,

    /// Timeout in milliseconds for entire path
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_max_depth() -> usize {
    10
}

fn default_timeout_ms() -> u64 {
    30000
}

/// A single step in a navigation path
/// Supports YAML format like:
/// ```yaml
/// - fetch:
///     follow_redirects: true
/// ```
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathStep {
    // ===== Basic Actions =====
    /// Fetch a URL
    Fetch(FetchStep),

    /// Extract data from response
    Extract(ExtractStep),

    /// Transform the current value
    Transform(TransformStep),

    /// Wait for a duration
    Wait(WaitStep),

    /// Return a result (success or error)
    Return(ReturnStep),

    // ===== Multi-Link Navigation =====
    /// Extract multiple links and process each
    ExtractAll(ExtractAllStep),

    // ===== Control Flow =====
    /// Conditional branching
    Branch(BranchStep),

    /// Loop until condition
    Loop(LoopStep),

    /// Try multiple approaches until one succeeds
    Try(TryStep),

    // ===== Reusability =====
    /// Use a named path
    Use(UsePathStep),

    // ===== Special =====
    /// WebView-based navigation
    WebView(WebViewStep),

    /// Auto-detect host and use appropriate path
    DetectHost(DetectHostStep),

    /// Set a variable for later use
    SetVar(SetVarStep),

    /// Log for debugging
    Log(LogStep),
}

// Custom deserializer to handle YAML map format: `- fetch: {key: value}`
impl<'de> Deserialize<'de> for PathStep {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct PathStepVisitor;

        impl<'de> Visitor<'de> for PathStepVisitor {
            type Value = PathStep;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map with a single key representing the step type")
            }

            fn visit_map<M>(self, mut map: M) -> Result<PathStep, M::Error>
            where
                M: MapAccess<'de>,
            {
                // Get the first (and should be only) key
                let key: String = map.next_key()?
                    .ok_or_else(|| de::Error::custom("expected a step type key"))?;

                let result = match key.as_str() {
                    "fetch" => {
                        let value: FetchStep = map.next_value()?;
                        Ok(PathStep::Fetch(value))
                    }
                    "extract" => {
                        let value: ExtractStep = map.next_value()?;
                        Ok(PathStep::Extract(value))
                    }
                    "transform" => {
                        let value: TransformStep = map.next_value()?;
                        Ok(PathStep::Transform(value))
                    }
                    "wait" => {
                        let value: WaitStep = map.next_value()?;
                        Ok(PathStep::Wait(value))
                    }
                    "return" => {
                        let value: ReturnStep = map.next_value()?;
                        Ok(PathStep::Return(value))
                    }
                    "extract_all" => {
                        let value: ExtractAllStep = map.next_value()?;
                        Ok(PathStep::ExtractAll(value))
                    }
                    "branch" => {
                        let value: BranchStep = map.next_value()?;
                        Ok(PathStep::Branch(value))
                    }
                    "loop" => {
                        let value: LoopStep = map.next_value()?;
                        Ok(PathStep::Loop(value))
                    }
                    "try" => {
                        let value: TryStep = map.next_value()?;
                        Ok(PathStep::Try(value))
                    }
                    "use" => {
                        let value: UsePathStep = map.next_value()?;
                        Ok(PathStep::Use(value))
                    }
                    "webview" => {
                        let value: WebViewStep = map.next_value()?;
                        Ok(PathStep::WebView(value))
                    }
                    "detect_host" => {
                        let value: DetectHostStep = map.next_value()?;
                        Ok(PathStep::DetectHost(value))
                    }
                    "set_var" => {
                        let value: SetVarStep = map.next_value()?;
                        Ok(PathStep::SetVar(value))
                    }
                    "log" => {
                        let value: LogStep = map.next_value()?;
                        Ok(PathStep::Log(value))
                    }
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["fetch", "extract", "transform", "wait", "return", "extract_all",
                          "branch", "loop", "try", "use", "webview", "detect_host", "set_var", "log"],
                    )),
                };

                result
            }
        }

        deserializer.deserialize_map(PathStepVisitor)
    }
}

// ============================================================================
// BASIC STEPS
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FetchStep {
    /// URL to fetch (if not provided, uses current URL)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// HTTP method (default: GET)
    #[serde(default = "default_method")]
    pub method: String,

    /// Request headers
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,

    /// Request body (for POST)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,

    /// Follow redirects
    #[serde(default = "default_true")]
    pub follow_redirects: bool,

    /// Timeout in ms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,

    /// Store response in variable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_as: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractStep {
    /// Extraction method
    pub method: PathExtractionMethod,

    /// Pattern (regex, selector, jsonpath, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    /// CSS selector (shorthand for method: selector)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,

    /// Regex pattern (shorthand for method: regex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,

    /// Attribute to extract (for selector method)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,

    /// Regex group to capture (default: 1)
    #[serde(default = "default_group")]
    pub group: usize,

    /// Store result in variable
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "as")]
    pub store_as: Option<String>,

    /// Fallback value if extraction fails
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<String>,

    /// Apply to URL instead of response body
    #[serde(default)]
    pub from_url: bool,
}

fn default_group() -> usize {
    1
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathExtractionMethod {
    Regex,
    Selector,
    JsonPath,
    XPath,
    Header,
    Text,
    Attribute,
}

impl Default for PathExtractionMethod {
    fn default() -> Self {
        PathExtractionMethod::Regex
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransformStep {
    /// Transformations to apply
    pub transformations: Vec<PathTransformation>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathTransformation {
    /// Replace using regex
    Replace { pattern: String, replacement: String },

    /// Template substitution
    Template { template: String },

    /// Prepend string
    Prepend { value: String },

    /// Append string
    Append { value: String },

    /// Trim whitespace
    Trim,

    /// URL encode
    UrlEncode,

    /// URL decode
    UrlDecode,

    /// Base64 decode
    Base64Decode,

    /// Extract from JSON
    JsonExtract { path: String },

    /// Join array with separator
    Join { separator: String },

    /// Split string
    Split { separator: String, index: Option<usize> },
}

// Custom deserializer for PathTransformation
impl<'de> Deserialize<'de> for PathTransformation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct PathTransformationVisitor;

        impl<'de> Visitor<'de> for PathTransformationVisitor {
            type Value = PathTransformation;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string for unit variants or a map for complex transformations")
            }

            // Handle unit variants as strings
            fn visit_str<E>(self, value: &str) -> Result<PathTransformation, E>
            where
                E: de::Error,
            {
                match value {
                    "trim" => Ok(PathTransformation::Trim),
                    "url_encode" => Ok(PathTransformation::UrlEncode),
                    "url_decode" => Ok(PathTransformation::UrlDecode),
                    "base64_decode" => Ok(PathTransformation::Base64Decode),
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["trim", "url_encode", "url_decode", "base64_decode"],
                    )),
                }
            }

            // Handle complex variants as maps
            fn visit_map<M>(self, mut map: M) -> Result<PathTransformation, M::Error>
            where
                M: MapAccess<'de>,
            {
                let key: String = map.next_key()?
                    .ok_or_else(|| de::Error::custom("expected a transformation type key"))?;

                match key.as_str() {
                    "replace" => {
                        #[derive(Deserialize)]
                        struct ReplaceData { pattern: String, replacement: String }
                        let data: ReplaceData = map.next_value()?;
                        Ok(PathTransformation::Replace { pattern: data.pattern, replacement: data.replacement })
                    }
                    "template" => {
                        #[derive(Deserialize)]
                        struct TemplateData { template: String }
                        let data: TemplateData = map.next_value()?;
                        Ok(PathTransformation::Template { template: data.template })
                    }
                    "prepend" => {
                        #[derive(Deserialize)]
                        struct PrependData { value: String }
                        let data: PrependData = map.next_value()?;
                        Ok(PathTransformation::Prepend { value: data.value })
                    }
                    "append" => {
                        #[derive(Deserialize)]
                        struct AppendData { value: String }
                        let data: AppendData = map.next_value()?;
                        Ok(PathTransformation::Append { value: data.value })
                    }
                    "json_extract" => {
                        #[derive(Deserialize)]
                        struct JsonExtractData { path: String }
                        let data: JsonExtractData = map.next_value()?;
                        Ok(PathTransformation::JsonExtract { path: data.path })
                    }
                    "join" => {
                        #[derive(Deserialize)]
                        struct JoinData { separator: String }
                        let data: JoinData = map.next_value()?;
                        Ok(PathTransformation::Join { separator: data.separator })
                    }
                    "split" => {
                        #[derive(Deserialize)]
                        struct SplitData { separator: String, index: Option<usize> }
                        let data: SplitData = map.next_value()?;
                        Ok(PathTransformation::Split { separator: data.separator, index: data.index })
                    }
                    // Unit variants in map form (for consistency)
                    "trim" => { let _: serde::de::IgnoredAny = map.next_value()?; Ok(PathTransformation::Trim) }
                    "url_encode" => { let _: serde::de::IgnoredAny = map.next_value()?; Ok(PathTransformation::UrlEncode) }
                    "url_decode" => { let _: serde::de::IgnoredAny = map.next_value()?; Ok(PathTransformation::UrlDecode) }
                    "base64_decode" => { let _: serde::de::IgnoredAny = map.next_value()?; Ok(PathTransformation::Base64Decode) }
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["replace", "template", "prepend", "append", "json_extract", "join", "split",
                          "trim", "url_encode", "url_decode", "base64_decode"],
                    )),
                }
            }
        }

        deserializer.deserialize_any(PathTransformationVisitor)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WaitStep {
    /// Milliseconds to wait
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ms: Option<u64>,

    /// Seconds to wait (alternative to ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<u64>,

    /// Wait for variable value (dynamic)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub var: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ReturnStep {
    /// Value to return (supports {var} interpolation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Return current value
    #[serde(default)]
    pub current: bool,

    /// Return as error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Mark as browser-only (cannot be downloaded directly)
    #[serde(default)]
    pub browser_only: bool,

    /// Reason for browser-only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Additional metadata to include
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

// ============================================================================
// MULTI-LINK NAVIGATION
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractAllStep {
    /// CSS selector for elements
    pub selector: String,

    /// Attribute to extract (default: href)
    #[serde(default = "default_href")]
    pub attribute: String,

    /// Maximum number of links to extract
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,

    /// Filter extracted URLs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<PathCondition>,

    /// How to process each link
    pub foreach: ForEachConfig,

    /// How to aggregate results
    #[serde(default)]
    pub aggregate: AggregateConfig,

    /// Extract metadata from each element
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extract_meta: HashMap<String, MetaExtraction>,
}

fn default_href() -> String {
    "href".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetaExtraction {
    /// CSS selector relative to parent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,

    /// Attribute to extract
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,

    /// Regex to apply
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,

    /// Static value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ForEachConfig {
    /// Conditional path selection based on URL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "match")]
    pub match_rules: Option<Vec<MatchRule>>,

    /// Single path for all (alternative to match)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathOrRef>,

    /// Default path if no match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<PathOrRef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MatchRule {
    /// Condition to match
    pub when: PathCondition,

    /// Path to use if condition matches
    pub path: PathOrRef,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PathOrRef {
    /// Reference to a named path
    Reference(UsePathStep),

    /// Inline path steps
    Inline(Vec<PathStep>),

    /// Full inline path with config
    InlineFull(Box<NavigationPath>),
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AggregateConfig {
    /// Aggregation mode
    #[serde(default)]
    pub mode: AggregateMode,

    /// Limit number of results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,

    /// Sort results by field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<String>,

    /// Group results by field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,

    /// Filter results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<PathCondition>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AggregateMode {
    /// Return all results
    #[default]
    All,

    /// Return first successful result
    FirstSuccess,

    /// Return results in priority order
    Priority,

    /// Run in parallel, return first to complete
    Fastest,

    /// Run in parallel, return all that complete within timeout
    Parallel,
}

// ============================================================================
// CONTROL FLOW
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BranchStep {
    /// Condition to evaluate
    #[serde(rename = "if")]
    pub condition: PathCondition,

    /// Steps if condition is true
    pub then: Vec<PathStep>,

    /// Steps if condition is false
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "else")]
    pub else_steps: Option<Vec<PathStep>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoopStep {
    /// Condition to continue loop
    #[serde(rename = "while")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub while_cond: Option<PathCondition>,

    /// Condition to exit loop
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<PathCondition>,

    /// Maximum iterations
    #[serde(default = "default_max_iterations")]
    pub max: usize,

    /// Steps to execute each iteration
    #[serde(rename = "do")]
    pub steps: Vec<PathStep>,
}

fn default_max_iterations() -> usize {
    10
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TryStep {
    /// List of attempts (paths) to try
    pub attempts: Vec<PathOrRef>,

    /// What to do if all attempts fail
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_all_fail: Option<Box<PathStep>>,
}

// ============================================================================
// CONDITIONS
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathCondition {
    // String conditions (on current URL or value)
    Contains(String),
    StartsWith(String),
    EndsWith(String),
    Matches(String), // Regex
    Equals(String),
    IsEmpty,
    NotEmpty,

    // HTTP response conditions
    Status(u16),
    StatusRange { min: u16, max: u16 },
    StatusSuccess, // 2xx
    StatusRedirect, // 3xx
    StatusError, // 4xx or 5xx

    // Header conditions
    HasHeader(String),
    HeaderEquals { name: String, value: String },
    HeaderContains { name: String, value: String },
    ContentType(String),

    // Response body conditions
    ResponseContains(String),
    ResponseMatches(String),
    SelectorExists(String),
    SelectorCount { selector: String, min: Option<usize>, max: Option<usize> },

    // Variable conditions
    VarEquals { var: String, value: String },
    VarContains { var: String, value: String },
    VarExists(String),
    VarMatches { var: String, pattern: String },

    // URL conditions
    UrlContains(String),
    UrlMatches(String),
    HostEquals(String),
    HostContains(String),

    // Logical operators
    And(Vec<PathCondition>),
    Or(Vec<PathCondition>),
    Not(Box<PathCondition>),

    // Always true/false
    Always,
    Never,
}

// Custom deserializer for PathCondition
// Supports both string format for unit variants: `not_empty`
// And map format for complex variants: `contains: "value"`
impl<'de> Deserialize<'de> for PathCondition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct PathConditionVisitor;

        impl<'de> Visitor<'de> for PathConditionVisitor {
            type Value = PathCondition;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string for unit variants or a map for complex conditions")
            }

            // Handle unit variants as strings: `not_empty`, `status_success`, etc.
            fn visit_str<E>(self, value: &str) -> Result<PathCondition, E>
            where
                E: de::Error,
            {
                match value {
                    "is_empty" => Ok(PathCondition::IsEmpty),
                    "not_empty" => Ok(PathCondition::NotEmpty),
                    "status_success" => Ok(PathCondition::StatusSuccess),
                    "status_redirect" => Ok(PathCondition::StatusRedirect),
                    "status_error" => Ok(PathCondition::StatusError),
                    "always" => Ok(PathCondition::Always),
                    "never" => Ok(PathCondition::Never),
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["is_empty", "not_empty", "status_success", "status_redirect",
                          "status_error", "always", "never"],
                    )),
                }
            }

            // Handle complex variants as maps
            fn visit_map<M>(self, mut map: M) -> Result<PathCondition, M::Error>
            where
                M: MapAccess<'de>,
            {
                let key: String = map.next_key()?
                    .ok_or_else(|| de::Error::custom("expected a condition type key"))?;

                match key.as_str() {
                    // String newtypes
                    "contains" => Ok(PathCondition::Contains(map.next_value()?)),
                    "starts_with" => Ok(PathCondition::StartsWith(map.next_value()?)),
                    "ends_with" => Ok(PathCondition::EndsWith(map.next_value()?)),
                    "matches" => Ok(PathCondition::Matches(map.next_value()?)),
                    "equals" => Ok(PathCondition::Equals(map.next_value()?)),
                    "has_header" => Ok(PathCondition::HasHeader(map.next_value()?)),
                    "content_type" => Ok(PathCondition::ContentType(map.next_value()?)),
                    "response_contains" => Ok(PathCondition::ResponseContains(map.next_value()?)),
                    "response_matches" => Ok(PathCondition::ResponseMatches(map.next_value()?)),
                    "selector_exists" => Ok(PathCondition::SelectorExists(map.next_value()?)),
                    "var_exists" => Ok(PathCondition::VarExists(map.next_value()?)),
                    "url_contains" => Ok(PathCondition::UrlContains(map.next_value()?)),
                    "url_matches" => Ok(PathCondition::UrlMatches(map.next_value()?)),
                    "host_equals" => Ok(PathCondition::HostEquals(map.next_value()?)),
                    "host_contains" => Ok(PathCondition::HostContains(map.next_value()?)),

                    // u16 newtype
                    "status" => Ok(PathCondition::Status(map.next_value()?)),

                    // Struct variants - parse as inline struct
                    "status_range" => {
                        #[derive(Deserialize)]
                        struct StatusRangeData { min: u16, max: u16 }
                        let data: StatusRangeData = map.next_value()?;
                        Ok(PathCondition::StatusRange { min: data.min, max: data.max })
                    }
                    "header_equals" => {
                        #[derive(Deserialize)]
                        struct HeaderEqualsData { name: String, value: String }
                        let data: HeaderEqualsData = map.next_value()?;
                        Ok(PathCondition::HeaderEquals { name: data.name, value: data.value })
                    }
                    "header_contains" => {
                        #[derive(Deserialize)]
                        struct HeaderContainsData { name: String, value: String }
                        let data: HeaderContainsData = map.next_value()?;
                        Ok(PathCondition::HeaderContains { name: data.name, value: data.value })
                    }
                    "selector_count" => {
                        #[derive(Deserialize)]
                        struct SelectorCountData { selector: String, min: Option<usize>, max: Option<usize> }
                        let data: SelectorCountData = map.next_value()?;
                        Ok(PathCondition::SelectorCount { selector: data.selector, min: data.min, max: data.max })
                    }
                    "var_equals" => {
                        #[derive(Deserialize)]
                        struct VarEqualsData { var: String, value: String }
                        let data: VarEqualsData = map.next_value()?;
                        Ok(PathCondition::VarEquals { var: data.var, value: data.value })
                    }
                    "var_contains" => {
                        #[derive(Deserialize)]
                        struct VarContainsData { var: String, value: String }
                        let data: VarContainsData = map.next_value()?;
                        Ok(PathCondition::VarContains { var: data.var, value: data.value })
                    }
                    "var_matches" => {
                        #[derive(Deserialize)]
                        struct VarMatchesData { var: String, pattern: String }
                        let data: VarMatchesData = map.next_value()?;
                        Ok(PathCondition::VarMatches { var: data.var, pattern: data.pattern })
                    }

                    // Recursive variants
                    "and" => Ok(PathCondition::And(map.next_value()?)),
                    "or" => Ok(PathCondition::Or(map.next_value()?)),
                    "not" => Ok(PathCondition::Not(map.next_value()?)),

                    other => Err(de::Error::unknown_variant(
                        other,
                        &["contains", "starts_with", "ends_with", "matches", "equals",
                          "status", "status_range", "has_header", "header_equals", "header_contains",
                          "content_type", "response_contains", "response_matches", "selector_exists",
                          "selector_count", "var_equals", "var_contains", "var_exists", "var_matches",
                          "url_contains", "url_matches", "host_equals", "host_contains",
                          "and", "or", "not"],
                    )),
                }
            }
        }

        deserializer.deserialize_any(PathConditionVisitor)
    }
}

// ============================================================================
// SPECIAL STEPS
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UsePathStep {
    /// Reference to named path (e.g., "paths.gofile")
    #[serde(rename = "use")]
    pub path_ref: String,

    /// Override variables
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub with: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebViewStep {
    /// CSS selector to wait for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_for: Option<String>,

    /// Timeout for wait (ms)
    #[serde(default = "default_webview_timeout")]
    pub timeout_ms: u64,

    /// Element to click
    #[serde(skip_serializing_if = "Option::is_none")]
    pub click: Option<String>,

    /// JavaScript to execute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute_js: Option<String>,

    /// Intercept download URL matching pattern
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intercept: Option<InterceptConfig>,

    /// Extract value after interaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract: Option<ExtractStep>,
}

fn default_webview_timeout() -> u64 {
    15000
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InterceptConfig {
    /// URL pattern to intercept (regex)
    pub pattern: String,

    /// Intercept type
    #[serde(default)]
    pub intercept_type: InterceptType,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InterceptType {
    #[default]
    Download,
    Navigation,
    Request,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DetectHostStep {
    /// Map of host patterns to paths
    pub hosts: HashMap<String, PathOrRef>,

    /// Default path if no host matches
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<PathOrRef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetVarStep {
    /// Variable name
    pub name: String,

    /// Value (supports {var} interpolation)
    pub value: String,

    /// Extract from response instead of static value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract: Option<ExtractStep>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogStep {
    /// Message to log
    pub message: String,

    /// Log level
    #[serde(default)]
    pub level: LogLevel,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    #[default]
    Debug,
    Info,
    Warn,
    Error,
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorHandler {
    /// Use another path
    Use(UsePathStep),

    /// Retry with delay
    Retry {
        max_attempts: usize,
        delay_ms: u64,
    },

    /// Return original URL
    ReturnOriginal,

    /// Return specific error
    ReturnError { message: String },

    /// Continue to next step (ignore error)
    Continue,
}

// ============================================================================
// RESULT TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedLink {
    /// The resolved URL
    pub url: String,

    /// Display label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Detected host name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,

    /// File size (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,

    /// Is browser-only (cannot direct download)
    #[serde(default)]
    pub browser_only: bool,

    /// Reason for browser-only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_only_reason: Option<String>,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,

    /// Debug: path taken to resolve this link
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolution_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResult {
    /// Successfully resolved links
    pub links: Vec<ResolvedLink>,

    /// Grouped links (if group_by was used)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<HashMap<String, Vec<ResolvedLink>>>,

    /// Errors encountered (non-fatal)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,

    /// Total time taken (ms)
    pub duration_ms: u64,
}

// ============================================================================
// PATH COLLECTION (for source config)
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PathsConfig {
    /// Named paths that can be referenced
    #[serde(default)]
    pub paths: HashMap<String, NavigationPath>,
}

impl NavigationPath {
    pub fn simple(steps: Vec<PathStep>) -> Self {
        NavigationPath {
            name: None,
            steps,
            on_error: None,
            max_depth: default_max_depth(),
            timeout_ms: default_timeout_ms(),
        }
    }
}
