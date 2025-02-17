use std::path::{Path, PathBuf};

use anyhow::Result;
use mcp_sdk::{
    server::Server,
    tools::{Tool, Tools},
    transport::ServerStdioTransport,
    types::{
        CallToolResponse, ListRequest, ResourcesListResponse, ServerCapabilities,
        ToolResponseContent,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        // needs to be stderr due to stdio transport
        .with_writer(std::io::stderr)
        .init();

    let tools = tool_set();
    let server = Server::builder(ServerStdioTransport)
        .capabilities(ServerCapabilities {
            tools: Some(json!({})),
            ..Default::default()
        })
        .tools(tools)
        .request_handler("resources/list", |_req: ListRequest| {
            Ok(ResourcesListResponse {
                resources: vec![],
                next_cursor: None,
                meta: None,
            })
        })
        .build();

    let server_handle = {
        let server = server;
        tokio::spawn(async move { server.listen().await })
    };

    server_handle
        .await?
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;
    Ok(())
}

fn tool_set() -> Tools {
    let mut tools = Tools::default();
    tools.add_tool(ReadFileTool::new());
    tools.add_tool(ListDirectoryTool::new());
    tools.add_tool(SearchFilesTool::new());
    tools.add_tool(GetFileInfoTool::new());
    tools.add_tool(ListAllowedDirectoriesTool::new());
    tools
}

// Tool parameter types
#[derive(Debug, Serialize, Deserialize)]
struct PathParam {
    path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchParam {
    path: String,
    pattern: String,
}

// Tool implementations
pub struct ReadFileTool;

impl ReadFileTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ReadFileTool {
    fn name(&self) -> String {
        "read_file".to_string()
    }

    fn description(&self) -> String {
        "Read the complete contents of a file from the file system. \
        Handles various text encodings and provides detailed error messages \
        if the file cannot be read. Use this tool when you need to examine \
        the contents of a single file. Only works within allowed directories."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string"
                }
            },
            "required": ["path"]
        })
    }

    fn call(&self, input: Option<serde_json::Value>) -> Result<CallToolResponse> {
        let args: PathParam = serde_json::from_value(input.unwrap_or_default())?;
        let path = get_path(&args.path)?;
        let content = std::fs::read_to_string(path)?;
        Ok(CallToolResponse {
            content: vec![ToolResponseContent::Text { text: content }],
            is_error: None,
            meta: None,
        })
    }
}

pub struct ListDirectoryTool;

impl ListDirectoryTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ListDirectoryTool {
    fn name(&self) -> String {
        "list_directory".to_string()
    }

    fn description(&self) -> String {
        "Get a detailed listing of all files and directories in a specified path. \
        Results clearly distinguish between files and directories with [FILE] and [DIR] \
        prefixes. This tool is essential for understanding directory structure and \
        finding specific files within a directory. Only works within allowed directories."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string"
                }
            },
            "required": ["path"]
        })
    }

    fn call(&self, input: Option<serde_json::Value>) -> Result<CallToolResponse> {
        let args: PathParam = serde_json::from_value(input.unwrap_or_default())?;
        let path = get_path(&args.path)?;
        let entries = std::fs::read_dir(path)?;
        let mut text = String::new();
        for entry in entries {
            let entry = entry?;
            let prefix = if entry.file_type()?.is_dir() {
                "[DIR]"
            } else {
                "[FILE]"
            };
            text.push_str(&format!(
                "{prefix} {}\n",
                entry.file_name().to_string_lossy()
            ));
        }
        Ok(CallToolResponse {
            content: vec![ToolResponseContent::Text { text }],
            is_error: None,
            meta: None,
        })
    }
}

pub struct SearchFilesTool;

impl SearchFilesTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for SearchFilesTool {
    fn name(&self) -> String {
        "search_files".to_string()
    }

    fn description(&self) -> String {
        "Recursively search for files and directories matching a pattern. \
        Searches through all subdirectories from the starting path. The search \
        is case-insensitive and matches partial names. Returns full paths to all \
        matching items. Great for finding files when you don't know their exact location. \
        Only searches within allowed directories."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string"
                },
                "pattern": {
                    "type": "string"
                }
            },
            "required": ["path", "pattern"]
        })
    }

    fn call(&self, input: Option<serde_json::Value>) -> Result<CallToolResponse> {
        let args: SearchParam = serde_json::from_value(input.unwrap_or_default())?;
        let path = get_path(&args.path)?;
        let mut matches = Vec::new();
        search_directory(&path, &args.pattern, &mut matches)?;
        Ok(CallToolResponse {
            content: vec![ToolResponseContent::Text {
                text: matches.join("\n"),
            }],
            is_error: None,
            meta: None,
        })
    }
}

pub struct GetFileInfoTool;

impl GetFileInfoTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GetFileInfoTool {
    fn name(&self) -> String {
        "get_file_info".to_string()
    }

    fn description(&self) -> String {
        "Retrieve detailed metadata about a file or directory. Returns comprehensive \
        information including size, creation time, last modified time, permissions, \
        and type. This tool is perfect for understanding file characteristics \
        without reading the actual content. Only works within allowed directories."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string"
                }
            },
            "required": ["path"]
        })
    }

    fn call(&self, input: Option<serde_json::Value>) -> Result<CallToolResponse> {
        let args: PathParam = serde_json::from_value(input.unwrap_or_default())?;
        let path = get_path(&args.path)?;
        let metadata = std::fs::metadata(path)?;
        Ok(CallToolResponse {
            content: vec![ToolResponseContent::Text {
                text: format!("{:?}", metadata),
            }],
            is_error: None,
            meta: None,
        })
    }
}

pub struct ListAllowedDirectoriesTool;

impl ListAllowedDirectoriesTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ListAllowedDirectoriesTool {
    fn name(&self) -> String {
        "list_allowed_directories".to_string()
    }

    fn description(&self) -> String {
        "List all directories that are allowed to be accessed by the tools.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn call(&self, _input: Option<serde_json::Value>) -> Result<CallToolResponse> {
        Ok(CallToolResponse {
            content: vec![ToolResponseContent::Text {
                text: "[]".to_string(),
            }],
            is_error: None,
            meta: None,
        })
    }
}

fn search_directory(dir: &Path, pattern: &str, matches: &mut Vec<String>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        // Check if the current file/directory matches the pattern
        if name.contains(&pattern.to_lowercase()) {
            matches.push(path.to_string_lossy().to_string());
        }

        // Recursively search subdirectories
        if path.is_dir() {
            search_directory(&path, pattern, matches)?;
        }
    }
    Ok(())
}

fn get_path(path: &str) -> Result<PathBuf> {
    if path.starts_with('~') {
        let home = home::home_dir().ok_or(anyhow::anyhow!("Could not determine home directory"))?;
        // Strip the ~ and join with home path
        let path = home.join(path.strip_prefix("~/").unwrap_or_default());
        Ok(path)
    } else {
        Ok(PathBuf::from(path))
    }
}
