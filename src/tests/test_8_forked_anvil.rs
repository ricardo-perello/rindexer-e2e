use anyhow::{Result, Context};
use tracing::info;
use crate::test_suite::TestContext;
use crate::tests::Test;
use std::time::Duration;
use tokio::time::sleep;

pub struct ForkedAnvilTest;

impl Test for ForkedAnvilTest {
    fn name(&self) -> &str {
        "test_8_forked_anvil"
    }
    
    fn description(&self) -> &str {
        "Test Rindexer with Anvil forked from Ethereum mainnet using real rindexer binary"
    }
    
    async fn run(&self, context: &mut TestContext) -> Result<()> {
        info!("Running Test 8: Forked Anvil Test");
        info!("Description: {}", self.description());
        
        // Clean up current Anvil and start a forked one
        info!("Cleaning up current Anvil instance...");
        context.cleanup().await?;
        
        // Start Anvil forked from Ethereum mainnet
        info!("Starting Anvil forked from Ethereum mainnet...");
        let forked_anvil = crate::anvil_setup::AnvilInstance::start_forked().await
            .context("Failed to start forked Anvil instance")?;
        
        // Update the test context with the forked anvil
        context.anvil = forked_anvil;
        info!("Forked Anvil ready at: {}", context.anvil.rpc_url);
        
        // Copy the anvil demo YAML file to our test project
        let demo_yaml_path = "test_examples/rindexer_demo_cli_anvil/rindexer.yaml";
        let target_yaml_path = context.project_path.join("rindexer.yaml");
        
        info!("Copying anvil demo YAML from: {}", demo_yaml_path);
        std::fs::copy(demo_yaml_path, &target_yaml_path)
            .context("Failed to copy demo YAML file")?;
        
        // Copy the SimpleERC20 ABI file
        let demo_abi_path = "abis/SimpleERC20.abi.json";
        let abis_dir = context.project_path.join("abis");
        std::fs::create_dir_all(&abis_dir)
            .context("Failed to create abis directory")?;
        
        let target_abi_path = abis_dir.join("SimpleERC20.abi.json");
        info!("Copying ABI file from: {}", demo_abi_path);
        std::fs::copy(demo_abi_path, &target_abi_path)
            .context("Failed to copy ABI file")?;
        
        info!("Created Rindexer project with forked Anvil at: {:?}", context.project_path);
        
        // Start Rindexer using the actual binary with 'start all'
        info!("Starting Rindexer with 'start all' command...");
        let rindexer_binary = "../rindexer/target/release/rindexer_cli";
        
        let mut rindexer = crate::rindexer_client::RindexerInstance::new(rindexer_binary, context.project_path.clone());
        rindexer.start_all().await
            .context("Failed to start Rindexer with 'start all'")?;
        
        info!("Rindexer process started successfully");
        
        // Wait a bit for Rindexer to start up
        info!("Waiting for Rindexer to start up...");
        sleep(Duration::from_secs(5)).await;
        
        // Test the health endpoint
        if let Some(health_client) = &context.health_client {
            info!("Testing health endpoint...");
            match health_client.get_health().await {
                Ok(health) => {
                    info!("✓ Health endpoint working: {:?}", health);
                    if health.status == "healthy" {
                        info!("✓ All services are healthy");
                    } else {
                        return Err(anyhow::anyhow!("Health endpoint reports unhealthy status: {}", health.status));
                    }
                }
                Err(_e) => {
                    info!("Health endpoint not ready yet, waiting...");
                    // Wait a bit more and try again
                    sleep(Duration::from_secs(10)).await;
                    match health_client.get_health().await {
                        Ok(health) => {
                            info!("✓ Health endpoint working after wait: {:?}", health);
                            if health.status != "healthy" {
                                return Err(anyhow::anyhow!("Health endpoint reports unhealthy status: {}", health.status));
                            }
                        }
                        Err(e2) => {
                            return Err(anyhow::anyhow!("Health endpoint check failed after wait: {}", e2));
                        }
                    }
                }
            }
        }
        
        // Check if Rindexer process is still running
        if !rindexer.is_running() {
            return Err(anyhow::anyhow!("Rindexer process is not running"));
        }
        info!("✓ Rindexer process is still running");
        
        // Test GraphQL endpoint if available
        info!("Testing GraphQL endpoint...");
        let graphql_url = "http://localhost:3001/graphql";
        match reqwest::get(graphql_url).await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("✓ GraphQL endpoint is responding");
                } else {
                    info!("GraphQL endpoint returned status: {}", response.status());
                }
            }
            Err(e) => {
                info!("GraphQL endpoint not ready yet: {}", e);
            }
        }
        
        // Clean up - stop the Rindexer process
        info!("Cleaning up Rindexer process...");
        let _ = rindexer.stop().await;
        
        info!("✓ Test 8 PASSED: Rindexer started successfully with forked Anvil and health monitoring");
        Ok(())
    }
}
