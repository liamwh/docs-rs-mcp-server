use anyhow::{anyhow, Context, Result};
use mcp_sdk::{
    tools::Tool,
    types::{CallToolResponse, ToolResponseContent},
};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info, warn};

/// Trait for fetching HTML content from a URL
#[async_trait::async_trait]
pub trait HtmlFetcher: Send + Sync {
    /// Fetches HTML content from a URL
    fn fetch_html(&self, url: &str) -> Result<String>;
}

/// Production implementation of HtmlFetcher that fetches from actual URLs
#[derive(Default)]
pub struct HttpHtmlFetcher {
    client: Client,
}

impl HttpHtmlFetcher {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl HtmlFetcher for HttpHtmlFetcher {
    fn fetch_html(&self, url: &str) -> Result<String> {
        debug!("Fetching HTML from URL: {}", url);
        let response = self
            .client
            .get(url)
            .send()
            .context(format!("Failed to fetch URL: {}", url))?;

        let status = response.status();
        debug!("Response status: {}", status);

        if !status.is_success() {
            error!("HTTP error response: {} for URL: {}", status, url);
            if let Ok(text) = response.text() {
                error!("Response body: {}", text);
            }
            return Err(anyhow!("Failed to fetch URL: HTTP {}", status));
        }

        let html = response
            .text()
            .context(format!("Failed to get text from response for URL: {}", url))?;

        debug!("Successfully fetched HTML ({} bytes)", html.len());
        Ok(html)
    }
}

#[cfg(test)]
pub struct TestHtmlFetcher;

#[cfg(test)]
impl HtmlFetcher for TestHtmlFetcher {
    fn fetch_html(&self, url: &str) -> Result<String> {
        debug!("TestHtmlFetcher: Fetching HTML from URL: {}", url);
        // Extract crate name and version from URL
        let parts: Vec<&str> = url.split('/').collect();
        let crate_name = parts
            .get(3)
            .ok_or_else(|| anyhow!("Invalid URL: missing crate name"))?;
        let version = parts
            .get(4)
            .ok_or_else(|| anyhow!("Invalid URL: missing version"))?;

        let file_type = if url.ends_with("all.html") {
            "all-items".to_string()
        } else {
            // Extract the struct name from the URL and convert to kebab case
            url.split('/')
                .last()
                .ok_or_else(|| anyhow!("Invalid URL: no path segments"))?
                .trim_end_matches(".html")
                .trim_start_matches("struct.")
                .to_lowercase()
                .replace('_', "-")
        };

        let test_file = format!(
            "test-data/get_struct_docs/{}-{}-{}.html",
            crate_name.replace('_', "-"),
            version,
            file_type
        );
        debug!("Attempting to read test file: {}", test_file);
        std::fs::read_to_string(&test_file)
            .context(format!("Failed to read test file: {}", test_file))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StructDocs {
    name: String,
    crate_name: String,
    description: String,
    methods: Vec<MethodDoc>,
    traits: Vec<String>,
    fields: Vec<FieldDoc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodDoc {
    name: String,
    signature: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldDoc {
    name: String,
    type_name: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StructDocsParams {
    crate_name: String,
    struct_name: String,
    version: Option<String>,
}

pub struct StructDocsTool {
    html_fetcher: Box<dyn HtmlFetcher>,
}

impl StructDocsTool {
    /// Creates a new instance of the StructDocsTool with the default production HTML fetcher.
    pub fn new() -> Self {
        Self {
            html_fetcher: Box::new(HttpHtmlFetcher::new()),
        }
    }

    /// Creates a new instance with a test fetcher for testing purposes.
    #[cfg(test)]
    pub fn new_with_test_fetcher() -> Self {
        debug!("Creating StructDocsTool with test fetcher");
        Self {
            html_fetcher: Box::new(TestHtmlFetcher),
        }
    }

    /// Gets the docs.rs URL, either from the environment variable DOCS_RS_URL or the default value.
    fn get_docs_rs_url(&self) -> String {
        std::env::var("DOCS_RS_URL").unwrap_or_else(|_| "https://docs.rs".to_string())
    }

    /// Fetches HTML content from a URL.
    fn fetch_html(&self, url: &str) -> Result<String> {
        self.html_fetcher.fetch_html(url)
    }

    fn find_struct_url(
        &self,
        crate_name: &str,
        struct_name: &str,
        version: Option<&str>,
    ) -> Result<String> {
        let version = version.unwrap_or("latest");
        let all_items_url = format!(
            "{}/{}/{}/{}/all.html",
            self.get_docs_rs_url(),
            crate_name,
            version,
            crate_name
        );
        debug!("Fetching all items from URL: {}", all_items_url);
        let html = self.fetch_html(&all_items_url)?;
        debug!("Successfully fetched all items HTML ({} bytes)", html.len());
        let document = Html::parse_document(&html);

        // Try both old and new docs.rs HTML structures
        let selectors = [
            "h3#structs + ul.all-items > li > a",
            "div[id='structs'] > div.item-table > div.item-row > a",
        ];

        // Extract the struct name without module path
        let struct_name_without_path = struct_name
            .split("::")
            .last()
            .ok_or_else(|| anyhow!("Invalid struct name: no parts found"))?;
        let module_path = struct_name
            .split("::")
            .take(struct_name.split("::").count() - 1)
            .collect::<Vec<_>>()
            .join("::");

        debug!(
            "Looking for struct: {} (without path: {}, module path: {})",
            struct_name, struct_name_without_path, module_path
        );

        for selector in &selectors {
            debug!("Trying selector: {}", selector);
            let link_selector = Selector::parse(selector)
                .map_err(|e| anyhow!("Failed to parse selector '{}': {}", selector, e))?;

            let mut found_links = Vec::new();
            for element in document.select(&link_selector) {
                let text = element.text().collect::<String>();
                let href = element.value().attr("href").unwrap_or_default();
                found_links.push(format!("text: '{}', href: '{}'", text, href));
            }
            debug!(
                "Found {} links with selector: {}",
                found_links.len(),
                selector
            );
            if !found_links.is_empty() {
                debug!("Links found:\n{}", found_links.join("\n"));
            }

            if let Some(struct_path) = document
                .select(&link_selector)
                .find(|element| {
                    let text = element.text().collect::<String>();
                    let href = element.value().attr("href").unwrap_or_default();
                    let matches_name = if module_path.is_empty() {
                        text == struct_name_without_path
                    } else {
                        text == struct_name
                            || text == format!("{}::{}", module_path, struct_name_without_path)
                    };
                    debug!(
                        "Checking link - text: '{}', href: '{}', matches_name: {}",
                        text, href, matches_name
                    );
                    matches_name && href.contains("struct")
                })
                .and_then(|element| element.value().attr("href"))
            {
                let base_url = format!(
                    "{}/{}/{}/{}",
                    self.get_docs_rs_url(),
                    crate_name,
                    version,
                    crate_name
                );
                debug!("Found struct path: {}", struct_path);
                if struct_path.starts_with("http") {
                    debug!("Using absolute URL: {}", struct_path);
                    return Ok(struct_path.to_string());
                } else {
                    // If we have a module path, we need to check if it's in the URL
                    let path_parts: Vec<&str> = struct_path.split('/').collect();
                    let mut final_path = struct_path.to_string();
                    if !module_path.is_empty()
                        && !path_parts.iter().any(|p| p.contains(&module_path))
                    {
                        // Insert the module path before the struct name
                        let last_slash = struct_path.rfind('/').unwrap_or(0);
                        final_path = format!(
                            "{}/{}/{}",
                            &struct_path[..last_slash],
                            module_path.replace("::", "/"),
                            &struct_path[last_slash + 1..]
                        );
                    }
                    let full_url = format!("{}{}", base_url, final_path);
                    debug!("Using constructed URL: {}", full_url);
                    return Ok(full_url);
                }
            }
        }

        error!(
            "Could not find struct {} in crate {} (version: {})",
            struct_name, crate_name, version
        );
        Err(anyhow!(
            "Could not find struct {} in crate {}",
            struct_name,
            crate_name
        ))
    }

    fn fetch_docs(
        &self,
        crate_name: &str,
        struct_name: &str,
        version: Option<&str>,
    ) -> Result<StructDocs> {
        info!(
            "Fetching docs for struct {} in crate {} (version: {:?})",
            struct_name, crate_name, version
        );

        // Find the correct URL for the struct
        let url = self.find_struct_url(crate_name, struct_name, version)?;
        debug!("Found struct URL: {}", url);

        let html = self.fetch_html(&url)?;
        debug!("Successfully fetched struct HTML ({} bytes)", html.len());
        let document = Html::parse_document(&html);

        // Parse main description
        let desc_selector = Selector::parse(".toggle.top-doc .docblock")
            .map_err(|e| anyhow!("Failed to parse description selector: {}", e))?;
        let description = document
            .select(&desc_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_default()
            .trim()
            .to_string();

        // Parse methods
        let method_selector = Selector::parse(".impl-items .toggle.method-toggle")
            .map_err(|e| anyhow!("Failed to parse method selector: {}", e))?;
        let fn_selector = Selector::parse(".code-header .fn")
            .map_err(|e| anyhow!("Failed to parse function name selector: {}", e))?;
        let code_header_selector = Selector::parse(".code-header")
            .map_err(|e| anyhow!("Failed to parse code header selector: {}", e))?;
        let docblock_selector = Selector::parse(".docblock")
            .map_err(|e| anyhow!("Failed to parse docblock selector: {}", e))?;

        let methods = document
            .select(&method_selector)
            .map(|method| {
                let name = method
                    .select(&fn_selector)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default()
                    .trim()
                    .to_string();

                let signature = method
                    .select(&code_header_selector)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default()
                    .trim()
                    .to_string();

                let description = method
                    .select(&docblock_selector)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default()
                    .trim()
                    .to_string();

                MethodDoc {
                    name,
                    signature,
                    description,
                }
            })
            .collect();

        // Extract trait implementations
        let mut traits: Vec<String> = Vec::new();

        // Parse selectors for trait implementations
        let trait_impl_selector = Selector::parse("#trait-implementations .impl")
            .map_err(|e| anyhow!("Failed to parse trait implementation selector: {}", e))?;
        let trait_name_selector = Selector::parse("h3 .trait")
            .map_err(|e| anyhow!("Failed to parse trait name selector: {}", e))?;

        // Check trait implementations
        for trait_section in document.select(&trait_impl_selector) {
            if let Some(trait_name) = trait_section.select(&trait_name_selector).next() {
                let trait_text = trait_name.text().collect::<String>();
                if !trait_text.is_empty() {
                    traits.push(trait_text);
                }
            }
        }

        // Check synthetic implementations
        let synthetic_impl_selector = Selector::parse("#synthetic-implementations .impl")
            .map_err(|e| anyhow!("Failed to parse synthetic implementation selector: {}", e))?;

        if traits.is_empty() {
            for synthetic_section in document.select(&synthetic_impl_selector) {
                if let Some(trait_name) = synthetic_section.select(&trait_name_selector).next() {
                    let trait_text = trait_name.text().collect::<String>();
                    if !trait_text.is_empty() {
                        traits.push(trait_text);
                    }
                }
            }
        }

        // Check blanket implementations
        let blanket_impl_selector = Selector::parse("#blanket-implementations .impl")
            .map_err(|e| anyhow!("Failed to parse blanket implementation selector: {}", e))?;

        if traits.is_empty() {
            for blanket_section in document.select(&blanket_impl_selector) {
                if let Some(trait_name) = blanket_section.select(&trait_name_selector).next() {
                    let trait_text = trait_name.text().collect::<String>();
                    if !trait_text.is_empty() {
                        traits.push(trait_text);
                    }
                }
            }
        }

        // Parse fields
        let field_selector = Selector::parse(".structfield")
            .map_err(|e| anyhow!("Failed to parse struct field selector: {}", e))?;
        let field_name_selector = Selector::parse(".structfield-name")
            .map_err(|e| anyhow!("Failed to parse field name selector: {}", e))?;
        let field_type_selector = Selector::parse(".type")
            .map_err(|e| anyhow!("Failed to parse field type selector: {}", e))?;

        let fields = document
            .select(&field_selector)
            .map(|field| {
                let name = field
                    .select(&field_name_selector)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default();

                let type_name = field
                    .select(&field_type_selector)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default();

                let description = field
                    .select(&docblock_selector)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default();

                FieldDoc {
                    name,
                    type_name,
                    description,
                }
            })
            .collect();

        Ok(StructDocs {
            name: struct_name.to_string(),
            crate_name: crate_name.to_string(),
            description,
            methods,
            traits,
            fields,
        })
    }
}

impl Default for StructDocsTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for StructDocsTool {
    fn name(&self) -> String {
        "get_struct_docs".to_string()
    }

    fn description(&self) -> String {
        "Fetches and parses documentation for a Rust struct from docs.rs".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["crate_name", "struct_name"],
            "properties": {
                "crate_name": {
                    "type": "string",
                    "description": "Name of the crate containing the struct"
                },
                "struct_name": {
                    "type": "string",
                    "description": "Name of the struct to look up"
                },
                "version": {
                    "type": "string",
                    "description": "Optional version of the crate. Defaults to latest if not specified"
                }
            }
        })
    }

    fn call(&self, input: Option<serde_json::Value>) -> Result<CallToolResponse> {
        let params: StructDocsParams = serde_json::from_value(input.unwrap_or_default())?;

        // Clone the parameters for the blocking task
        let crate_name = params.crate_name.clone();
        let struct_name = params.struct_name.clone();
        let version = params.version.clone();

        // Run the blocking HTTP requests in a blocking task
        let docs = tokio::task::block_in_place(|| {
            self.fetch_docs(&crate_name, &struct_name, version.as_deref())
        })?;

        Ok(CallToolResponse {
            content: vec![ToolResponseContent::Text {
                text: serde_json::to_string_pretty(&docs)?,
            }],
            is_error: None,
            meta: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_find_struct_url() -> Result<()> {
        let tool = StructDocsTool::new_with_test_fetcher();

        // Test with exact name
        let url =
            tool.find_struct_url("opentelemetry_sdk", "TracerProviderBuilder", Some("0.28.0"))?;
        assert!(
            url.contains("opentelemetry_sdk/trace/struct.TracerProviderBuilder.html"),
            "URL should contain correct path"
        );

        // Test with module path
        let url = tool.find_struct_url(
            "opentelemetry_sdk",
            "trace::TracerProviderBuilder",
            Some("0.28.0"),
        )?;
        assert!(
            url.contains("opentelemetry_sdk/trace/struct.TracerProviderBuilder.html"),
            "URL should contain correct path"
        );

        Ok(())
    }

    #[test]
    fn test_fetch_docs() -> Result<()> {
        let tool = StructDocsTool::new_with_test_fetcher();

        // Test with exact name
        let docs = tool.fetch_docs("opentelemetry_sdk", "TracerProviderBuilder", Some("0.28.0"))?;
        assert_eq!(docs.name, "TracerProviderBuilder", "Wrong struct name");
        assert_eq!(docs.crate_name, "opentelemetry_sdk", "Wrong crate name");
        assert!(!docs.description.is_empty(), "Should have a description");
        assert!(!docs.traits.is_empty(), "Should have traits");

        // Test with module path
        let docs = tool.fetch_docs(
            "opentelemetry_sdk",
            "trace::TracerProviderBuilder",
            Some("0.28.0"),
        )?;
        assert_eq!(
            docs.name, "trace::TracerProviderBuilder",
            "Wrong struct name"
        );
        assert_eq!(docs.crate_name, "opentelemetry_sdk", "Wrong crate name");
        assert!(!docs.description.is_empty(), "Should have a description");
        assert!(!docs.traits.is_empty(), "Should have traits");

        Ok(())
    }
}
