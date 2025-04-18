use anyhow::Result;
use docs_rs_mcp::{
    tools::{CrateInfoTool, CrateItemsTool},
    StructDocsTool,
};
use mcp_sdk::tools::{Tool, Tools};

#[test]
fn test_server_capabilities() -> Result<()> {
    // Set up the tools
    let mut tools = Tools::default();
    tools.add_tool(CrateInfoTool::new());
    tools.add_tool(CrateItemsTool::new());
    tools.add_tool(StructDocsTool::new());

    // Verify tool definitions
    let crate_info = CrateInfoTool::new().as_definition();
    let crate_items = CrateItemsTool::new().as_definition();
    let struct_docs = StructDocsTool::new().as_definition();

    assert!(
        crate_info.name.contains("crate_info"),
        "Tool name should contain crate_info"
    );
    assert!(
        crate_items.name.contains("crate_items"),
        "Tool name should contain crate_items"
    );
    assert!(
        crate_info.description.is_some(),
        "Tool should have a description"
    );
    assert!(
        crate_items.description.is_some(),
        "Tool should have a description"
    );
    assert!(
        !crate_info.description.as_ref().unwrap().is_empty(),
        "Tool description should not be empty"
    );
    assert!(
        !crate_items.description.as_ref().unwrap().is_empty(),
        "Tool description should not be empty"
    );
    assert!(
        !struct_docs.description.as_ref().unwrap().is_empty(),
        "Tool description should not be empty"
    );

    Ok(())
}
