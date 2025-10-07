use anyhow::{Result, Context};
use tracing::info;
use std::pin::Pin;
use std::future::Future;

use crate::test_suite::TestContext;
use crate::tests::registry::{TestDefinition, TestModule};

pub struct DemoYamlTests;

impl TestModule for DemoYamlTests {
    fn get_tests() -> Vec<TestDefinition> {
        vec![
            TestDefinition::new(
                "test_6_demo_yaml",
                "Test Rindexer with the demo YAML configuration adapted for Anvil",
                demo_yaml_test,
            ).with_timeout(180),
        ]
    }
}

fn demo_yaml_test(context: &mut TestContext) -> Pin<Box<dyn Future<Output = Result<()>> + '_>> {
    Box::pin(async move {
        info!("Running Test 6: Demo YAML Test");
    
        // Copy and modify the anvil demo YAML file to our test project
        let demo_yaml_path = "test_examples/rindexer_demo_cli_anvil/rindexer.yaml";
        let target_yaml_path = context.project_path.join("rindexer.yaml");
        
        info!("Copying anvil demo YAML from: {}", demo_yaml_path);
        let demo_yaml_content = std::fs::read_to_string(demo_yaml_path)
            .context("Failed to read demo YAML file")?;
        
        // Update the YAML to use our test contract and disable PostgreSQL
        let updated_yaml = demo_yaml_content
            .replace("RocketPoolETH", "SimpleERC20")
            .replace("./abis/RocketTokenRETH.abi.json", "./abis/SimpleERC20.abi.json")
            .replace("0xae78736cd615f374d3085123a210448e74fc6393", "0x5FbDB2315678afecb367f032d93F642f64180aa3")
            .replace("18600000", "0")
            .replace("18600500", "0")  // Set end_block to 0 to match latest block
            .replace("postgres:\n    enabled: true\n    drop_each_run: true", "postgres:\n    enabled: false")
            .replace("csv:\n    enabled: false", "csv:\n    enabled: true");
        
        std::fs::write(&target_yaml_path, updated_yaml)
            .context("Failed to write updated YAML file")?;
        
        // Copy the SimpleERC20 ABI file
        let demo_abi_path = "abis/SimpleERC20.abi.json";
        let abis_dir = context.project_path.join("abis");
        std::fs::create_dir_all(&abis_dir)
            .context("Failed to create abis directory")?;
        
        let target_abi_path = abis_dir.join("SimpleERC20.abi.json");
        info!("Copying ABI file from: {}", demo_abi_path);
        std::fs::copy(demo_abi_path, &target_abi_path)
            .context("Failed to copy ABI file")?;
        
        info!("Created Rindexer project with demo YAML at: {:?}", context.project_path);
        
        // Start Rindexer with the demo configuration
        info!("Starting Rindexer with demo configuration...");
        let mut rindexer = crate::rindexer_client::RindexerInstance::new(&context.rindexer_binary, context.project_path.clone());
        rindexer.start_indexer().await
            .context("Failed to start Rindexer instance")?;
        
        context.rindexer = Some(rindexer);
        info!("Rindexer started successfully");
        
        // Wait for Rindexer to start up
        info!("Waiting for Rindexer to be ready...");
        context.wait_for_health_ready(30).await?;
        info!("Rindexer is ready");
        
        // Verify Rindexer is still running
        if !context.is_rindexer_running() {
            return Err(anyhow::anyhow!("Rindexer process is not running"));
        }
        
        info!("✓ Test 6 PASSED: Rindexer started successfully with demo YAML");
        Ok(())
    })
}
