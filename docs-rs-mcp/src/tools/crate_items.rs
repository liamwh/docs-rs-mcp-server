use anyhow::Result;
use mcp_sdk::{
    tools::Tool,
    types::{CallToolResponse, ToolResponseContent},
};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct CrateItems {
    crate_name: String,
    version: String,
    items: HashMap<String, Vec<Item>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    name: String,
    path: String,
    doc_link: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CrateNameParam {
    crate_name: String,
    version: Option<String>,
}

pub struct CrateItemsTool;

impl CrateItemsTool {
    pub fn new() -> Self {
        Self
    }

    fn scrape_items(&self, crate_name: &str, version: Option<&str>) -> Result<CrateItems> {
        let client = Client::new();
        let version = version.unwrap_or("latest");
        let url = format!("https://docs.rs/{crate_name}/{version}/{crate_name}/all.html");

        let response = client.get(&url).send()?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch docs.rs page: {} - {}",
                response.status(),
                url
            ));
        }

        let html = response.text()?;
        let document = Html::parse_document(&html);

        // Initialize our categorized items
        let mut items: HashMap<String, Vec<Item>> = HashMap::new();

        // The sections we want to extract
        let sections = [
            "macros",
            "structs",
            "enums",
            "traits",
            "functions",
            "types",
            "attributes",
        ];

        for section in sections {
            // Each section has an h3 with the section ID and a following ul.all-items
            let section_name = match section {
                "types" => "Type Aliases".to_string(),
                s => {
                    let mut capitalized =
                        s.chars().next().unwrap().to_uppercase().collect::<String>();
                    capitalized.push_str(&s[1..]);
                    capitalized
                }
            };

            // Select all items in this section using the h3 ID and following ul.all-items
            let selector = format!("h3#{} + ul.all-items > li > a", section);
            let link_selector = Selector::parse(&selector).unwrap();

            let mut section_items = Vec::new();
            for link in document.select(&link_selector) {
                let name = link.text().collect::<String>().trim().to_string();
                let path = link
                    .value()
                    .attr("href")
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                let doc_link = if path.starts_with("http") {
                    path.clone()
                } else {
                    format!(
                        "https://docs.rs/{}/{}/{}/{}",
                        crate_name,
                        version,
                        crate_name,
                        path.trim_start_matches('/')
                    )
                };

                if !name.is_empty() && !path.is_empty() {
                    section_items.push(Item {
                        name,
                        path,
                        doc_link,
                    });
                }
            }

            if !section_items.is_empty() {
                items.insert(section_name, section_items);
            }
        }

        Ok(CrateItems {
            crate_name: crate_name.to_string(),
            version: version.to_string(),
            items,
        })
    }
}

impl Default for CrateItemsTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for CrateItemsTool {
    fn name(&self) -> String {
        "crate_items".to_string()
    }

    fn description(&self) -> String {
        "Get a list of all items (structs, traits, enums, etc.) exposed by a crate \
        by scraping its docs.rs documentation. Returns categorized items with their \
        documentation links."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "crate_name": {
                    "type": "string",
                    "description": "Name of the crate to get items for"
                },
                "version": {
                    "type": "string",
                    "description": "Optional version of the crate (defaults to latest)"
                }
            },
            "required": ["crate_name"]
        })
    }

    fn call(&self, input: Option<serde_json::Value>) -> Result<CallToolResponse> {
        let args: CrateNameParam = serde_json::from_value(input.unwrap_or_default())?;
        let items = self.scrape_items(&args.crate_name, args.version.as_deref())?;

        Ok(CallToolResponse {
            content: vec![ToolResponseContent::Text {
                text: serde_json::to_string_pretty(&items)?,
            }],
            is_error: None,
            meta: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use scraper::Html;
    use std::fs;

    fn load_scraper_test_html() -> String {
        fs::read_to_string("test-data/list-of-all-items-scraper-0.22.0.html")
            .expect("Should be able to read test HTML file")
    }

    fn load_tokio_test_html() -> String {
        fs::read_to_string("test-data/list-of-all-items-tokio-1.43.0.html")
            .expect("Should be able to read test HTML file")
    }

    #[test]
    fn test_parse_html() {
        let html = load_scraper_test_html();
        let document = Html::parse_document(&html);

        // Test section headers
        let sections = ["structs", "enums", "traits", "types"];
        for section in &sections {
            let selector = format!("h3#{}", section);
            let section_header = document.select(&Selector::parse(&selector).unwrap()).next();
            assert!(
                section_header.is_some(),
                "Should find section header for {}",
                section
            );
        }

        // Test section names and content
        for section in &sections {
            let selector = format!("#{} + ul.all-items", section);
            let items = document.select(&Selector::parse(&selector).unwrap()).next();
            assert!(
                items.is_some(),
                "Should find items list for {} section",
                section
            );
        }

        // Test link selector and content for structs
        let struct_links: Vec<_> = document
            .select(&Selector::parse("#structs + ul.all-items > li > a").unwrap())
            .collect();

        let struct_names: Vec<_> = struct_links
            .iter()
            .map(|link| link.text().collect::<String>())
            .collect();

        // Verify specific structs we know should be there
        assert!(struct_names.contains(&"element_ref::ElementRef".to_string()));
        assert!(struct_names.contains(&"html::Html".to_string()));
        assert!(struct_names.contains(&"selector::Selector".to_string()));
        assert!(struct_names.contains(&"selector::CssLocalName".to_string()));

        // Verify total count of structs
        assert_eq!(struct_links.len(), 18, "Should find 18 structs");
    }

    #[test]
    fn test_link_formatting() {
        let html = load_scraper_test_html();
        let document = Html::parse_document(&html);

        // Test all item links
        for section in ["structs", "enums", "traits", "types"] {
            let selector = format!("#{} + ul.all-items > li > a", section);
            let link_selector = Selector::parse(&selector).unwrap();

            for link in document.select(&link_selector) {
                let href = link.value().attr("href").unwrap();
                let name = link.text().collect::<String>();

                // Verify link format
                assert!(
                    href.ends_with(".html"),
                    "Link should end with .html: {}",
                    href
                );

                // Verify link matches the expected pattern for the item type
                match section {
                    "structs" => assert!(
                        href.contains("struct."),
                        "Struct link wrong format: {}",
                        href
                    ),
                    "enums" => assert!(href.contains("enum."), "Enum link wrong format: {}", href),
                    "traits" => {
                        assert!(href.contains("trait."), "Trait link wrong format: {}", href)
                    }
                    "types" => assert!(href.contains("type."), "Type link wrong format: {}", href),
                    _ => unreachable!(),
                }

                // Verify name is not empty
                assert!(!name.is_empty(), "Link text should not be empty");
            }
        }
    }

    #[test]
    fn test_item_counts() {
        let html = load_scraper_test_html();
        let document = Html::parse_document(&html);

        // Verify exact counts for each category
        let counts = [("structs", 18), ("enums", 5), ("traits", 3), ("types", 3)];

        for (section, expected_count) in counts {
            let selector = format!("#{} + ul.all-items > li", section);
            let items = document
                .select(&Selector::parse(&selector).unwrap())
                .count();
            assert_eq!(
                items, expected_count,
                "Wrong number of items in {} section",
                section
            );
        }
    }

    #[test]
    fn test_item_paths() {
        let html = load_scraper_test_html();
        let document = Html::parse_document(&html);

        // Test some specific items and their full paths
        let expected_items = [
            (
                "element_ref::ElementRef",
                "element_ref/struct.ElementRef.html",
            ),
            ("html::Html", "html/struct.Html.html"),
            ("CaseSensitivity", "enum.CaseSensitivity.html"),
            ("Element", "trait.Element.html"),
            ("StrTendril", "type.StrTendril.html"),
        ];

        for (name, expected_path) in expected_items {
            let found = document
                .select(&Selector::parse("ul.all-items > li > a").unwrap())
                .any(|link| {
                    link.text().collect::<String>() == name
                        && link.value().attr("href").unwrap().ends_with(expected_path)
                });

            assert!(
                found,
                "Could not find item {} with path ending in {}",
                name, expected_path
            );
        }
    }

    #[test]
    fn test_tokio_items() {
        let html = load_tokio_test_html();
        let document = Html::parse_document(&html);

        // Test section headers - tokio has additional sections like macros and functions
        let sections = [
            "macros",
            "structs",
            "enums",
            "traits",
            "functions",
            "types",
            "attributes",
        ];
        for section in &sections {
            let selector = format!("h3#{}", section);
            let section_header = document.select(&Selector::parse(&selector).unwrap()).next();
            assert!(
                section_header.is_some(),
                "Should find section header for {}",
                section
            );
        }

        // Test some specific items from each category
        let expected_items = [
            // Structs
            ("fs::File", "fs/struct.File.html"),
            ("io::BufReader", "io/struct.BufReader.html"),
            ("net::TcpListener", "net/struct.TcpListener.html"),
            ("sync::Mutex", "sync/struct.Mutex.html"),
            ("time::Interval", "time/struct.Interval.html"),
            // Enums
            ("runtime::RuntimeFlavor", "runtime/enum.RuntimeFlavor.html"),
            ("sync::TryAcquireError", "sync/enum.TryAcquireError.html"),
            // Traits
            ("io::AsyncRead", "io/trait.AsyncRead.html"),
            ("io::AsyncWrite", "io/trait.AsyncWrite.html"),
            // Functions
            ("fs::read", "fs/fn.read.html"),
            ("time::sleep", "time/fn.sleep.html"),
            // Macros
            ("select", "macro.select.html"),
            ("join", "macro.join.html"),
            // Types
            ("net::unix::uid_t", "net/unix/type.uid_t.html"),
        ];

        for (name, expected_path) in expected_items {
            let found = document
                .select(&Selector::parse("ul.all-items > li > a").unwrap())
                .any(|link| {
                    let link_text = link.text().collect::<String>();
                    let href = link.value().attr("href").unwrap();
                    link_text == name && href.ends_with(expected_path)
                });

            assert!(
                found,
                "Could not find item {} with path ending in {}",
                name, expected_path
            );
        }

        // Verify counts for some categories
        let category_counts = [
            ("Structs", 155),    // Structs
            ("Enums", 16),       // Core enums
            ("Traits", 17),      // Core traits
            ("Functions", 67),   // Various utility functions
            ("Macros", 5),       // Core macros
            ("Type Aliases", 8), // Type aliases
        ];

        for (category, expected_count) in category_counts {
            let selector = match category {
                "Type Aliases" => "h3#types + ul.all-items > li",
                _ => &format!(
                    "h3#{} + ul.all-items > li",
                    category.to_lowercase().replace(' ', "")
                ),
            };
            let category_items = document.select(&Selector::parse(selector).unwrap()).count();

            assert_eq!(
                category_items, expected_count,
                "Wrong number of items in {} category",
                category
            );
        }

        // Test link format
        let link_selector = Selector::parse("ul.all-items > li > a").unwrap();
        let all_links = document.select(&link_selector);
        for link in all_links {
            let href = link.value().attr("href").unwrap();
            assert!(
                href.ends_with(".html"),
                "Link should end with .html: {}",
                href
            );
            assert!(
                !href.contains("//"),
                "Link should not contain double slashes: {}",
                href
            );
        }
    }
}
