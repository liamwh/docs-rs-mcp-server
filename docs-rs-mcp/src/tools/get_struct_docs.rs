use anyhow::{anyhow, Result};
use mcp_sdk::{
    tools::Tool,
    types::{CallToolResponse, ToolResponseContent},
};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::form_urlencoded;

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

pub struct StructDocsTool;

impl StructDocsTool {
    pub fn new() -> Self {
        Self
    }

    fn fetch_docs(
        &self,
        crate_name: &str,
        struct_name: &str,
        version: Option<&str>,
    ) -> Result<StructDocs> {
        let client = Client::new();
        // URL encode the struct name to handle special characters
        let encoded_struct_name =
            form_urlencoded::byte_serialize(struct_name.as_bytes()).collect::<String>();

        // Get the version to use
        let version = match version {
            Some(v) => {
                // Verify the version exists
                let version_url = format!("https://docs.rs/{}/{}", crate_name, v);
                let response = client.get(&version_url).send()?;
                if !response.status().is_success() {
                    return Err(anyhow!("Version {} not found for crate {}", v, crate_name));
                }
                v.to_string()
            }
            None => {
                // Get the latest version
                let latest_url = format!("https://docs.rs/{}/latest", crate_name);
                let response = client.get(&latest_url).send()?;
                if !response.status().is_success() {
                    return Err(anyhow!(
                        "Failed to fetch crate info for {}: HTTP {}",
                        crate_name,
                        response.status()
                    ));
                }
                // The actual URL might be a redirect, get it from the response URL
                response
                    .url()
                    .path_segments()
                    .and_then(|segments| segments.last())
                    .ok_or_else(|| anyhow!("Could not determine crate version"))?
                    .to_string()
            }
        };

        // Try different module paths for the struct
        let possible_paths = vec![
            format!("struct.{}.html", encoded_struct_name),
            format!("rt/struct.{}.html", encoded_struct_name), // for tokio::Runtime
            format!("de/struct.{}.html", encoded_struct_name), // for serde::Deserializer
            format!("ser/struct.{}.html", encoded_struct_name), // for serde::Serializer
        ];

        for path in possible_paths {
            let url = format!(
                "https://docs.rs/{}/{}/{}/{}",
                crate_name, version, crate_name, path
            );

            let response = client.get(&url).send()?;
            if response.status().is_success() {
                let text = response.text()?;
                let document = Html::parse_document(&text);

                // Parse main description
                let desc_selector = Selector::parse(".toggle.top-doc .docblock").unwrap();
                let description = document
                    .select(&desc_selector)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default()
                    .trim()
                    .to_string();

                // If we found the struct page (it has a description or at least a title)
                if !description.is_empty()
                    || document.select(&Selector::parse("h1.fqn").unwrap()).count() > 0
                {
                    // Parse methods
                    let method_selector =
                        Selector::parse(".impl-items .toggle.method-toggle").unwrap();
                    let methods = document
                        .select(&method_selector)
                        .map(|method| {
                            let name = method
                                .select(&Selector::parse(".code-header .fn").unwrap())
                                .next()
                                .map(|el| el.text().collect::<String>())
                                .unwrap_or_default()
                                .trim()
                                .to_string();

                            let signature = method
                                .select(&Selector::parse(".code-header").unwrap())
                                .next()
                                .map(|el| el.text().collect::<String>())
                                .unwrap_or_default()
                                .trim()
                                .to_string();

                            let description = method
                                .select(&Selector::parse(".docblock").unwrap())
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

                    // Check trait implementations
                    for trait_section in
                        document.select(&Selector::parse("#trait-implementations .impl").unwrap())
                    {
                        if let Some(trait_name) = trait_section
                            .select(&Selector::parse("h3 .trait").unwrap())
                            .next()
                        {
                            let trait_text = trait_name.text().collect::<String>();
                            if !trait_text.is_empty() {
                                traits.push(trait_text);
                            }
                        }
                    }

                    // Check synthetic implementations
                    if traits.is_empty() {
                        for synthetic_section in document
                            .select(&Selector::parse("#synthetic-implementations .impl").unwrap())
                        {
                            if let Some(trait_name) = synthetic_section
                                .select(&Selector::parse("h3 .trait").unwrap())
                                .next()
                            {
                                let trait_text = trait_name.text().collect::<String>();
                                if !trait_text.is_empty() {
                                    traits.push(trait_text);
                                }
                            }
                        }
                    }

                    // Check blanket implementations
                    if traits.is_empty() {
                        for blanket_section in document
                            .select(&Selector::parse("#blanket-implementations .impl").unwrap())
                        {
                            if let Some(trait_name) = blanket_section
                                .select(&Selector::parse("h3 .trait").unwrap())
                                .next()
                            {
                                let trait_text = trait_name.text().collect::<String>();
                                if !trait_text.is_empty() {
                                    traits.push(trait_text);
                                }
                            }
                        }
                    }

                    // If still no traits found, try a more general selector
                    if traits.is_empty() {
                        for impl_section in document.select(&Selector::parse(".impl").unwrap()) {
                            let impl_text = impl_section.text().collect::<String>();
                            if impl_text.contains("impl") {
                                traits.push(impl_text.trim().to_string());
                            }
                        }
                    }

                    // Parse fields
                    let field_selector = Selector::parse(".structfield").unwrap();
                    let fields = document
                        .select(&field_selector)
                        .map(|field| {
                            let name = field
                                .select(&Selector::parse(".structfield-name").unwrap())
                                .next()
                                .map(|el| el.text().collect::<String>())
                                .unwrap_or_default();

                            let type_name = field
                                .select(&Selector::parse(".type").unwrap())
                                .next()
                                .map(|el| el.text().collect::<String>())
                                .unwrap_or_default();

                            let description = field
                                .select(&Selector::parse(".docblock").unwrap())
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

                    return Ok(StructDocs {
                        name: struct_name.to_string(),
                        crate_name: crate_name.to_string(),
                        description,
                        methods,
                        traits,
                        fields,
                    });
                }
            }
        }

        // If we get here, we couldn't find the struct in any of the tried paths
        Err(anyhow!(
            "Could not find documentation for {}::{}",
            crate_name,
            struct_name
        ))
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
