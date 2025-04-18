use anyhow::Result;
use mcp_sdk::{
    server::Server,
    tools::{Tool, Tools},
    transport::ServerStdioTransport,
    types::{ListRequest, ResourcesListResponse, ServerCapabilities},
};
use serde_json::json;

mod tools;
use tools::{CrateInfoTool, CrateItemsTool, StructDocsTool};

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
            tools: Some(json!({
                "crate_info": CrateInfoTool::new().as_definition(),
                "crate_items": CrateItemsTool::new().as_definition(),
                "get_struct_docs": StructDocsTool::new().as_definition(),
            })),
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

//NOTE: Must be updated if a tool is added!
fn tool_set() -> Tools {
    let mut tools = Tools::default();
    tools.add_tool(CrateInfoTool::new());
    tools.add_tool(CrateItemsTool::new());
    tools.add_tool(StructDocsTool::new());
    tools
}
