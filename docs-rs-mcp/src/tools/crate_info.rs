use anyhow::Result;
use mcp_sdk::{
    tools::Tool,
    types::{CallToolResponse, ToolResponseContent},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct CrateInfo {
    name: String,
    description: String,
    version: String,
    license: Option<String>,
    rust_version: Option<String>,
    documentation: Option<String>,
    homepage: Option<String>,
    repository: Option<String>,
    crates_io: Option<String>,
    features: Vec<Feature>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Feature {
    name: String,
    is_default: bool,
    dependencies: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CrateNameParam {
    crate_name: String,
}

pub struct CrateInfoTool;

impl CrateInfoTool {
    pub fn new() -> Self {
        Self
    }

    fn parse_cargo_info_output(&self, output: &str) -> Result<CrateInfo> {
        let mut lines = output.lines();

        // First line contains name and tags
        let first_line = lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty output"))?;
        let name = first_line
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string();

        // Second line is description
        let description = lines.next().unwrap_or_default().trim().to_string();

        let mut info = CrateInfo {
            name,
            description,
            version: String::new(),
            license: None,
            rust_version: None,
            documentation: None,
            homepage: None,
            repository: None,
            crates_io: None,
            features: Vec::new(),
        };

        let mut in_features = false;
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with("features:") {
                in_features = true;
                continue;
            }

            if in_features {
                if line.starts_with("note:") {
                    break;
                }

                let mut parts = line.splitn(2, '=');
                let name = parts.next().unwrap_or_default().trim().to_string();
                let deps_str = parts.next().unwrap_or_default().trim();

                let is_default = name.starts_with('+');
                let name = name.trim_start_matches('+').to_string();

                let dependencies = if deps_str.is_empty() {
                    Vec::new()
                } else {
                    deps_str
                        .trim_matches(|c| c == '[' || c == ']')
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect()
                };

                info.features.push(Feature {
                    name,
                    is_default,
                    dependencies,
                });
            } else if let Some((key, value)) = line.split_once(':') {
                let value = value.trim();
                match key.trim() {
                    "version" => info.version = value.to_string(),
                    "license" => info.license = Some(value.to_string()),
                    "rust-version" => info.rust_version = Some(value.to_string()),
                    "documentation" => info.documentation = Some(value.to_string()),
                    "homepage" => info.homepage = Some(value.to_string()),
                    "repository" => info.repository = Some(value.to_string()),
                    "crates.io" => info.crates_io = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        Ok(info)
    }
}

impl Tool for CrateInfoTool {
    fn name(&self) -> String {
        "crate_info".to_string()
    }

    fn description(&self) -> String {
        "Get detailed information about a Rust crate using cargo-info. \
        Returns strongly typed information including version, license, \
        documentation links, and feature flags."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "crate_name": {
                    "type": "string",
                    "description": "Name of the crate to get information about"
                }
            },
            "required": ["crate_name"]
        })
    }

    fn call(&self, input: Option<serde_json::Value>) -> Result<CallToolResponse> {
        let args: CrateNameParam = serde_json::from_value(input.unwrap_or_default())?;

        let output = Command::new("cargo")
            .arg("info")
            .arg(&args.crate_name)
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to get crate info: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let output_str = String::from_utf8(output.stdout)?;
        let crate_info = self.parse_cargo_info_output(&output_str)?;

        Ok(CallToolResponse {
            content: vec![ToolResponseContent::Text {
                text: serde_json::to_string_pretty(&crate_info)?,
            }],
            is_error: None,
            meta: None,
        })
    }
}
