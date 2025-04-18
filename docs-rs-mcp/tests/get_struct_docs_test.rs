use anyhow::{anyhow, Context, Result};
use docs_rs_mcp::tools::get_struct_docs::StructDocsTool;
use mcp_sdk::tools::Tool;
use serde_json::json;
use tracing::{debug, error, info};

#[test_log::test]
fn test_get_struct_docs() -> Result<()> {
    info!("Starting test_get_struct_docs");
    debug!("Attempting to create StructDocsTool with test fetcher");
    let tool = StructDocsTool::new_with_test_fetcher();

    // Test with latest version
    let input = json!({
        "crate_name": "surrealdb",
        "struct_name": "Surreal"
    });

    debug!("Testing with latest version: {:?}", input);
    let result = tool
        .call(Some(input.clone()))
        .context("Failed to call tool with latest version")?;
    verify_struct_docs(&result)?;

    // Test with specific version
    let input_with_version = json!({
        "crate_name": "surrealdb",
        "struct_name": "Surreal",
        "version": "2.2.0"
    });

    debug!("Testing with specific version: {:?}", input_with_version);
    let result_with_version = tool
        .call(Some(input_with_version))
        .context("Failed to call tool with specific version")?;
    verify_struct_docs(&result_with_version)?;

    info!("Test completed successfully");
    Ok(())
}

fn verify_struct_docs(result: &mcp_sdk::types::CallToolResponse) -> Result<()> {
    debug!("Verifying struct docs response");
    if let mcp_sdk::types::ToolResponseContent::Text { text } = &result.content[0] {
        let docs: serde_json::Value =
            serde_json::from_str(text).context("Failed to parse response JSON")?;
        debug!(
            "Parsed response JSON: {}",
            serde_json::to_string_pretty(&docs)?
        );

        let docs = docs
            .as_object()
            .ok_or_else(|| anyhow!("Response is not a JSON object"))?;

        let name = docs["name"]
            .as_str()
            .ok_or_else(|| anyhow!("Name field is not a string"))?;
        let crate_name = docs["crate_name"]
            .as_str()
            .ok_or_else(|| anyhow!("Crate name field is not a string"))?;
        let description = docs["description"]
            .as_str()
            .ok_or_else(|| anyhow!("Description field is not a string"))?;

        debug!(
            "Verifying basic fields - name: {}, crate_name: {}",
            name, crate_name
        );
        assert_eq!(name, "Surreal", "Wrong struct name");
        assert_eq!(crate_name, "surrealdb", "Wrong crate name");
        assert!(!description.is_empty(), "Description should not be empty");

        // Verify we have methods
        let methods = docs["methods"]
            .as_array()
            .ok_or_else(|| anyhow!("Methods field is not an array"))?;
        debug!("Found {} methods", methods.len());
        assert!(!methods.is_empty(), "Should have at least one method");

        // Check first method has required fields
        let first_method = &methods[0];
        let method_name = first_method["name"]
            .as_str()
            .ok_or_else(|| anyhow!("Method name is not a string"))?;
        let signature = first_method["signature"]
            .as_str()
            .ok_or_else(|| anyhow!("Method signature is not a string"))?;
        let method_desc = first_method["description"]
            .as_str()
            .ok_or_else(|| anyhow!("Method description is not a string"))?;

        debug!("Verifying first method - name: {}", method_name);
        assert!(!method_name.is_empty(), "Method name should not be empty");
        assert!(
            !signature.is_empty(),
            "Method signature should not be empty"
        );
        assert!(
            !method_desc.is_empty(),
            "Method description should not be empty"
        );

        // Verify traits
        let traits = docs["traits"]
            .as_array()
            .ok_or_else(|| anyhow!("Traits field is not an array"))?;
        debug!("Found {} traits", traits.len());
        assert!(!traits.is_empty(), "Should have at least one trait");

        // Verify fields if any exist
        let fields = docs["fields"]
            .as_array()
            .ok_or_else(|| anyhow!("Fields field is not an array"))?;
        debug!("Found {} fields", fields.len());
        if !fields.is_empty() {
            let first_field = &fields[0];
            let field_name = first_field["name"]
                .as_str()
                .ok_or_else(|| anyhow!("Field name is not a string"))?;
            let type_name = first_field["type_name"]
                .as_str()
                .ok_or_else(|| anyhow!("Field type is not a string"))?;

            debug!(
                "Verifying first field - name: {}, type: {}",
                field_name, type_name
            );
            assert!(!field_name.is_empty(), "Field name should not be empty");
            assert!(!type_name.is_empty(), "Field type should not be empty");
        }

        debug!("Struct docs verification completed successfully");
        Ok(())
    } else {
        error!("Expected Text response but got something else");
        Err(anyhow!("Expected Text response"))
    }
}
