//! Document Text Extractors
//!
//! Provides extractors for various document formats to convert them into
//! plain text for further processing.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

use crate::{IngestionError, Result};

/// Result of text extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// Extracted text content
    pub text: String,
    /// Detected content type
    pub content_type: String,
    /// Extracted metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Character encoding used
    pub encoding: String,
    /// Whether extraction was complete or partial
    pub complete: bool,
    /// Warnings during extraction
    pub warnings: Vec<String>,
}

impl ExtractionResult {
    pub fn new(text: impl Into<String>, content_type: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            content_type: content_type.into(),
            metadata: HashMap::new(),
            encoding: "utf-8".to_string(),
            complete: true,
            warnings: Vec::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    pub fn partial(mut self) -> Self {
        self.complete = false;
        self
    }
}

/// Trait for document text extractors
#[async_trait]
pub trait TextExtractor: Send + Sync {
    /// Extract text from document content
    async fn extract(&self, content: &[u8], filename: Option<&str>) -> Result<ExtractionResult>;

    /// Get supported content types
    fn supported_types(&self) -> Vec<&'static str>;

    /// Check if this extractor can handle the content type
    fn can_handle(&self, content_type: &str) -> bool {
        self.supported_types().iter().any(|&t| {
            content_type.starts_with(t) || content_type.contains(t)
        })
    }

    /// Get extractor name
    fn name(&self) -> &'static str;
}

/// Plain text extractor
pub struct PlainTextExtractor {
    /// Maximum content size to process
    max_size: usize,
}

impl PlainTextExtractor {
    pub fn new() -> Self {
        Self {
            max_size: 10 * 1024 * 1024, // 10MB default
        }
    }

    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }
}

impl Default for PlainTextExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TextExtractor for PlainTextExtractor {
    async fn extract(&self, content: &[u8], _filename: Option<&str>) -> Result<ExtractionResult> {
        if content.len() > self.max_size {
            return Err(IngestionError::ExtractionFailed(format!(
                "Content too large: {} bytes (max {})",
                content.len(),
                self.max_size
            )));
        }

        // Try UTF-8 first
        let (text, encoding) = match std::str::from_utf8(content) {
            Ok(s) => (s.to_string(), "utf-8"),
            Err(_) => {
                // Try detecting encoding
                let (decoded, actual_encoding, had_errors) =
                    encoding_rs::WINDOWS_1252.decode(content);
                if had_errors {
                    (decoded.into_owned(), "windows-1252-lossy")
                } else {
                    (decoded.into_owned(), actual_encoding.name())
                }
            }
        };

        let mut result = ExtractionResult::new(text, "text/plain");
        result.encoding = encoding.to_string();

        debug!(
            encoding = %encoding,
            size = content.len(),
            "Extracted plain text"
        );

        Ok(result)
    }

    fn supported_types(&self) -> Vec<&'static str> {
        vec!["text/plain", "text/x-"]
    }

    fn name(&self) -> &'static str {
        "plain_text"
    }
}

/// Markdown extractor (preserves structure)
pub struct MarkdownExtractor {
    /// Whether to extract headings as metadata
    extract_headings: bool,
    /// Whether to extract links as metadata
    extract_links: bool,
    /// Whether to strip markdown syntax
    strip_syntax: bool,
}

impl MarkdownExtractor {
    pub fn new() -> Self {
        Self {
            extract_headings: true,
            extract_links: true,
            strip_syntax: false,
        }
    }

    pub fn with_strip_syntax(mut self, strip: bool) -> Self {
        self.strip_syntax = strip;
        self
    }

    /// Extract headings from markdown content
    fn extract_headings_from_text(&self, text: &str) -> Vec<String> {
        text.lines()
            .filter(|line| line.starts_with('#'))
            .map(|line| line.trim_start_matches('#').trim().to_string())
            .collect()
    }

    /// Extract links from markdown content
    fn extract_links_from_text(&self, text: &str) -> Vec<(String, String)> {
        let mut links = Vec::new();
        let mut remaining = text;

        while let Some(start) = remaining.find('[') {
            remaining = &remaining[start..];

            if let Some(mid) = remaining.find("](") {
                let label = &remaining[1..mid];
                remaining = &remaining[mid + 2..];

                if let Some(end) = remaining.find(')') {
                    let url = &remaining[..end];
                    links.push((label.to_string(), url.to_string()));
                    remaining = &remaining[end + 1..];
                }
            } else {
                remaining = &remaining[1..];
            }
        }

        links
    }

    /// Strip markdown syntax
    fn strip_markdown(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Remove code blocks
        while let Some(start) = result.find("```") {
            if let Some(end) = result[start + 3..].find("```") {
                result.replace_range(start..start + 3 + end + 3, "");
            } else {
                break;
            }
        }

        // Remove inline code
        result = result.replace('`', "");

        // Remove headers markers
        result = result
            .lines()
            .map(|line| line.trim_start_matches('#').trim())
            .collect::<Vec<_>>()
            .join("\n");

        // Remove bold/italic
        result = result.replace("**", "").replace("__", "").replace('*', "").replace('_', "");

        // Remove links, keep text
        while let Some(start) = result.find('[') {
            if let Some(mid) = result[start..].find("](") {
                if let Some(end) = result[start + mid..].find(')') {
                    let label = &result[start + 1..start + mid].to_string();
                    result.replace_range(start..start + mid + end + 1, label);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        result
    }
}

impl Default for MarkdownExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TextExtractor for MarkdownExtractor {
    async fn extract(&self, content: &[u8], filename: Option<&str>) -> Result<ExtractionResult> {
        let text = std::str::from_utf8(content).map_err(|e| {
            IngestionError::EncodingError(format!("Invalid UTF-8 in markdown: {}", e))
        })?;

        let processed = if self.strip_syntax {
            self.strip_markdown(text)
        } else {
            text.to_string()
        };

        let mut result = ExtractionResult::new(processed, "text/markdown");

        if self.extract_headings {
            let headings = self.extract_headings_from_text(text);
            if !headings.is_empty() {
                result.metadata.insert(
                    "headings".to_string(),
                    serde_json::json!(headings),
                );
            }
        }

        if self.extract_links {
            let links = self.extract_links_from_text(text);
            if !links.is_empty() {
                result.metadata.insert(
                    "links".to_string(),
                    serde_json::json!(links.iter().map(|(l, u)| {
                        serde_json::json!({"label": l, "url": u})
                    }).collect::<Vec<_>>()),
                );
            }
        }

        if let Some(name) = filename {
            result.metadata.insert(
                "filename".to_string(),
                serde_json::json!(name),
            );
        }

        debug!(
            headings_count = result.metadata.get("headings").map(|h| {
                h.as_array().map(|a| a.len()).unwrap_or(0)
            }).unwrap_or(0),
            "Extracted markdown"
        );

        Ok(result)
    }

    fn supported_types(&self) -> Vec<&'static str> {
        vec!["text/markdown", "text/x-markdown"]
    }

    fn name(&self) -> &'static str {
        "markdown"
    }
}

/// JSON extractor
pub struct JsonExtractor {
    /// Whether to flatten nested objects
    flatten: bool,
    /// Fields to extract text from
    text_fields: Vec<String>,
    /// Pretty-print JSON in output
    pretty: bool,
}

impl JsonExtractor {
    pub fn new() -> Self {
        Self {
            flatten: false,
            text_fields: vec![
                "text".to_string(),
                "content".to_string(),
                "body".to_string(),
                "description".to_string(),
                "message".to_string(),
            ],
            pretty: false,
        }
    }

    pub fn with_flatten(mut self, flatten: bool) -> Self {
        self.flatten = flatten;
        self
    }

    pub fn with_text_fields(mut self, fields: Vec<String>) -> Self {
        self.text_fields = fields;
        self
    }

    /// Extract text from JSON value
    fn extract_text_from_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Array(arr) => {
                arr.iter()
                    .map(|v| self.extract_text_from_value(v))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            serde_json::Value::Object(obj) => {
                // First try to find known text fields
                for field in &self.text_fields {
                    if let Some(v) = obj.get(field) {
                        if let serde_json::Value::String(s) = v {
                            return s.clone();
                        }
                    }
                }

                // Otherwise concatenate all string values
                obj.values()
                    .filter_map(|v| {
                        if let serde_json::Value::String(s) = v {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            serde_json::Value::Null => String::new(),
        }
    }

    /// Flatten nested JSON object
    fn flatten_json(
        &self,
        value: &serde_json::Value,
        prefix: &str,
        result: &mut HashMap<String, serde_json::Value>,
    ) {
        match value {
            serde_json::Value::Object(obj) => {
                for (k, v) in obj {
                    let new_key = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", prefix, k)
                    };
                    self.flatten_json(v, &new_key, result);
                }
            }
            serde_json::Value::Array(arr) => {
                for (i, v) in arr.iter().enumerate() {
                    let new_key = format!("{}[{}]", prefix, i);
                    self.flatten_json(v, &new_key, result);
                }
            }
            _ => {
                result.insert(prefix.to_string(), value.clone());
            }
        }
    }
}

impl Default for JsonExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TextExtractor for JsonExtractor {
    async fn extract(&self, content: &[u8], _filename: Option<&str>) -> Result<ExtractionResult> {
        let text = std::str::from_utf8(content).map_err(|e| {
            IngestionError::EncodingError(format!("Invalid UTF-8 in JSON: {}", e))
        })?;

        let parsed: serde_json::Value = serde_json::from_str(text).map_err(|e| {
            IngestionError::ExtractionFailed(format!("Invalid JSON: {}", e))
        })?;

        let extracted_text = self.extract_text_from_value(&parsed);

        let output = if self.pretty {
            serde_json::to_string_pretty(&parsed).unwrap_or(text.to_string())
        } else {
            extracted_text.clone()
        };

        let mut result = ExtractionResult::new(output, "application/json");

        if self.flatten {
            let mut flattened = HashMap::new();
            self.flatten_json(&parsed, "", &mut flattened);
            result.metadata.insert(
                "flattened".to_string(),
                serde_json::json!(flattened),
            );
        }

        // Extract some basic metadata
        if let serde_json::Value::Object(obj) = &parsed {
            result.metadata.insert(
                "field_count".to_string(),
                serde_json::json!(obj.len()),
            );
        }

        Ok(result)
    }

    fn supported_types(&self) -> Vec<&'static str> {
        vec!["application/json", "text/json"]
    }

    fn name(&self) -> &'static str {
        "json"
    }
}

/// Code file extractor
pub struct CodeExtractor {
    /// Languages and their comment patterns
    comment_patterns: HashMap<String, (Vec<&'static str>, Option<(&'static str, &'static str)>)>,
    /// Whether to extract comments separately
    extract_comments: bool,
}

impl CodeExtractor {
    pub fn new() -> Self {
        let mut patterns = HashMap::new();

        // (single-line prefixes, optional (block_start, block_end))
        patterns.insert(
            "rust".to_string(),
            (vec!["//"], Some(("/*", "*/"))),
        );
        patterns.insert(
            "python".to_string(),
            (vec!["#"], Some(("\"\"\"", "\"\"\""))),
        );
        patterns.insert(
            "javascript".to_string(),
            (vec!["//"], Some(("/*", "*/"))),
        );
        patterns.insert(
            "typescript".to_string(),
            (vec!["//"], Some(("/*", "*/"))),
        );
        patterns.insert(
            "java".to_string(),
            (vec!["//"], Some(("/*", "*/"))),
        );
        patterns.insert(
            "c".to_string(),
            (vec!["//"], Some(("/*", "*/"))),
        );
        patterns.insert(
            "cpp".to_string(),
            (vec!["//"], Some(("/*", "*/"))),
        );
        patterns.insert(
            "go".to_string(),
            (vec!["//"], Some(("/*", "*/"))),
        );
        patterns.insert("ruby".to_string(), (vec!["#"], Some(("=begin", "=end"))));
        patterns.insert("shell".to_string(), (vec!["#"], None));
        patterns.insert("yaml".to_string(), (vec!["#"], None));

        Self {
            comment_patterns: patterns,
            extract_comments: true,
        }
    }

    /// Detect language from filename
    fn detect_language(&self, filename: Option<&str>) -> Option<String> {
        let ext = filename?.rsplit('.').next()?;

        match ext.to_lowercase().as_str() {
            "rs" => Some("rust".to_string()),
            "py" => Some("python".to_string()),
            "js" | "mjs" | "cjs" => Some("javascript".to_string()),
            "ts" | "tsx" => Some("typescript".to_string()),
            "java" => Some("java".to_string()),
            "c" | "h" => Some("c".to_string()),
            "cpp" | "cc" | "cxx" | "hpp" => Some("cpp".to_string()),
            "go" => Some("go".to_string()),
            "rb" => Some("ruby".to_string()),
            "sh" | "bash" => Some("shell".to_string()),
            "yml" | "yaml" => Some("yaml".to_string()),
            _ => None,
        }
    }

    /// Extract comments from code
    fn extract_comments_from_code(&self, code: &str, language: &str) -> Vec<String> {
        let mut comments = Vec::new();

        let patterns = match self.comment_patterns.get(language) {
            Some(p) => p,
            None => return comments,
        };

        let (single_prefixes, block_pattern) = patterns;

        for line in code.lines() {
            let trimmed = line.trim();
            for prefix in single_prefixes {
                if trimmed.starts_with(prefix) {
                    let comment = trimmed[prefix.len()..].trim();
                    if !comment.is_empty() {
                        comments.push(comment.to_string());
                    }
                }
            }
        }

        // Extract block comments
        if let Some((start, end)) = block_pattern {
            let mut remaining = code;
            while let Some(s) = remaining.find(start) {
                remaining = &remaining[s + start.len()..];
                if let Some(e) = remaining.find(end) {
                    let comment = remaining[..e].trim();
                    if !comment.is_empty() {
                        comments.push(comment.to_string());
                    }
                    remaining = &remaining[e + end.len()..];
                } else {
                    break;
                }
            }
        }

        comments
    }
}

impl Default for CodeExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TextExtractor for CodeExtractor {
    async fn extract(&self, content: &[u8], filename: Option<&str>) -> Result<ExtractionResult> {
        let text = std::str::from_utf8(content).map_err(|e| {
            IngestionError::EncodingError(format!("Invalid UTF-8 in code: {}", e))
        })?;

        let language = self.detect_language(filename);
        let content_type = format!(
            "text/x-{}",
            language.as_deref().unwrap_or("plain")
        );

        let mut result = ExtractionResult::new(text, content_type);

        if let Some(ref lang) = language {
            result.metadata.insert(
                "language".to_string(),
                serde_json::json!(lang),
            );

            if self.extract_comments {
                let comments = self.extract_comments_from_code(text, lang);
                if !comments.is_empty() {
                    result.metadata.insert(
                        "comments".to_string(),
                        serde_json::json!(comments),
                    );
                }
            }
        }

        // Count lines
        let line_count = text.lines().count();
        result.metadata.insert(
            "line_count".to_string(),
            serde_json::json!(line_count),
        );

        Ok(result)
    }

    fn supported_types(&self) -> Vec<&'static str> {
        vec![
            "text/x-rust",
            "text/x-python",
            "text/x-java",
            "text/x-c",
            "text/x-c++",
            "text/x-go",
            "text/x-javascript",
            "text/x-typescript",
            "text/x-ruby",
            "text/x-shellscript",
            "text/x-yaml",
            "application/x-yaml",
        ]
    }

    fn name(&self) -> &'static str {
        "code"
    }
}

/// Registry of text extractors
pub struct ExtractorRegistry {
    extractors: Vec<Arc<dyn TextExtractor>>,
    default_extractor: Arc<dyn TextExtractor>,
}

impl ExtractorRegistry {
    pub fn new() -> Self {
        Self {
            extractors: Vec::new(),
            default_extractor: Arc::new(PlainTextExtractor::new()),
        }
    }

    /// Create with default extractors
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(PlainTextExtractor::new()));
        registry.register(Arc::new(MarkdownExtractor::new()));
        registry.register(Arc::new(JsonExtractor::new()));
        registry.register(Arc::new(CodeExtractor::new()));
        registry
    }

    /// Register an extractor
    pub fn register(&mut self, extractor: Arc<dyn TextExtractor>) {
        self.extractors.push(extractor);
    }

    /// Get extractor for content type
    pub fn get_extractor(&self, content_type: &str) -> Arc<dyn TextExtractor> {
        for extractor in &self.extractors {
            if extractor.can_handle(content_type) {
                return extractor.clone();
            }
        }
        self.default_extractor.clone()
    }

    /// Get extractor by filename
    pub fn get_by_filename(&self, filename: &str) -> Arc<dyn TextExtractor> {
        let content_type = mime_guess::from_path(filename)
            .first_or_text_plain()
            .to_string();
        self.get_extractor(&content_type)
    }

    /// List all registered extractors
    pub fn list(&self) -> Vec<&'static str> {
        self.extractors.iter().map(|e| e.name()).collect()
    }
}

impl Default for ExtractorRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plain_text_extractor() {
        let extractor = PlainTextExtractor::new();
        let content = b"Hello, world!";

        let result = extractor.extract(content, None).await.unwrap();

        assert_eq!(result.text, "Hello, world!");
        assert_eq!(result.content_type, "text/plain");
        assert!(result.complete);
    }

    #[tokio::test]
    async fn test_markdown_extractor() {
        let extractor = MarkdownExtractor::new();
        let content = b"# Heading 1\n\nSome text.\n\n## Heading 2\n\n[Link](http://example.com)";

        let result = extractor.extract(content, Some("test.md")).await.unwrap();

        assert!(result.metadata.contains_key("headings"));
        assert!(result.metadata.contains_key("links"));

        let headings = result.metadata["headings"].as_array().unwrap();
        assert_eq!(headings.len(), 2);
    }

    #[tokio::test]
    async fn test_markdown_strip_syntax() {
        let extractor = MarkdownExtractor::new().with_strip_syntax(true);
        let content = b"# Title\n\n**Bold** and *italic*";

        let result = extractor.extract(content, None).await.unwrap();

        assert!(!result.text.contains('#'));
        assert!(!result.text.contains('*'));
    }

    #[tokio::test]
    async fn test_json_extractor() {
        let extractor = JsonExtractor::new();
        let content = br#"{"text": "Hello", "count": 42}"#;

        let result = extractor.extract(content, None).await.unwrap();

        assert!(result.text.contains("Hello"));
        assert!(result.metadata.contains_key("field_count"));
    }

    #[tokio::test]
    async fn test_json_extractor_flatten() {
        let extractor = JsonExtractor::new().with_flatten(true);
        let content = br#"{"nested": {"key": "value"}}"#;

        let result = extractor.extract(content, None).await.unwrap();

        assert!(result.metadata.contains_key("flattened"));
    }

    #[tokio::test]
    async fn test_code_extractor() {
        let extractor = CodeExtractor::new();
        let content = b"// Comment\nfn main() {\n    println!(\"Hello\");\n}";

        let result = extractor.extract(content, Some("test.rs")).await.unwrap();

        assert_eq!(
            result.metadata["language"].as_str().unwrap(),
            "rust"
        );
        assert!(result.metadata.contains_key("comments"));
        assert!(result.metadata.contains_key("line_count"));
    }

    #[tokio::test]
    async fn test_extractor_registry() {
        let registry = ExtractorRegistry::with_defaults();

        let md_extractor = registry.get_by_filename("test.md");
        assert_eq!(md_extractor.name(), "markdown");

        let json_extractor = registry.get_extractor("application/json");
        assert_eq!(json_extractor.name(), "json");

        let list = registry.list();
        assert!(list.len() >= 4);
    }

    #[tokio::test]
    async fn test_extraction_result_builder() {
        let result = ExtractionResult::new("content", "text/plain")
            .with_metadata("key", serde_json::json!("value"))
            .with_warning("Warning message")
            .partial();

        assert_eq!(result.text, "content");
        assert!(result.metadata.contains_key("key"));
        assert_eq!(result.warnings.len(), 1);
        assert!(!result.complete);
    }
}
