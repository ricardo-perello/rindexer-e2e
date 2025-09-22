use anyhow::{Result, Context};
use tracing::info;
use crate::test_suite::TestSuite;
use crate::tests::TestCaseImpl;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

pub struct ForkedAnvilTest;

impl TestCaseImpl for ForkedAnvilTest {
    fn name(&self) -> &str {
        "test_8_forked_anvil"
    }
    
    fn description(&self) -> &str {
        "Test Rindexer with Anvil forked from Ethereum mainnet using real rindexer binary"
    }
    
    async fn run(&self, test_suite: &mut TestSuite) -> Result<()> {
        info!("Running Test 8: Forked Anvil Test");
        info!("Description: {}", self.description());
        
        // Clean up current Anvil and start a forked one
        info!("Cleaning up current Anvil instance...");
        test_suite.cleanup().await?;
        
        // Start Anvil forked from Ethereum mainnet
        info!("Starting Anvil forked from Ethereum mainnet...");
        let forked_anvil = crate::anvil_setup::AnvilInstance::start_forked().await
            .context("Failed to start forked Anvil instance")?;
        
        // Update the test suite with the forked anvil
        test_suite.anvil = forked_anvil;
        info!("Forked Anvil ready at: {}", test_suite.anvil.rpc_url);
        
        // Copy the anvil demo YAML file to our test project
        let demo_yaml_path = "test_examples/rindexer_demo_cli_anvil/rindexer.yaml";
        let target_yaml_path = test_suite.project_path.join("rindexer.yaml");
        
        info!("Copying anvil demo YAML from: {}", demo_yaml_path);
        std::fs::copy(demo_yaml_path, &target_yaml_path)
            .context("Failed to copy demo YAML file")?;
        
        // Copy the SimpleERC20 ABI file
        let demo_abi_path = "abis/SimpleERC20.abi.json";
        let abis_dir = test_suite.project_path.join("abis");
        std::fs::create_dir_all(&abis_dir)
            .context("Failed to create abis directory")?;
        
        let target_abi_path = abis_dir.join("SimpleERC20.abi.json");
        info!("Copying ABI file from: {}", demo_abi_path);
        std::fs::copy(demo_abi_path, &target_abi_path)
            .context("Failed to copy ABI file")?;
        
        info!("Created Rindexer project with forked Anvil at: {:?}", test_suite.project_path);
        
        // Start Rindexer using the actual binary with 'start all'
        info!("Starting Rindexer with 'start all' command...");
        let rindexer_binary = "../rindexer/target/release/rindexer_cli";
        
        let mut cmd = Command::new(rindexer_binary);
        cmd.current_dir(&test_suite.project_path)
           .arg("start")
           .arg("all");
        
        let mut child = cmd.spawn()
            .context("Failed to start Rindexer with 'start all'")?;
        
        info!("Rindexer process started with PID: {:?}", child.id());
        
        // Wait a bit for Rindexer to start up
        info!("Waiting for Rindexer to start up...");
        sleep(Duration::from_secs(5)).await;
        
        // Test the health endpoint
        if let Some(health_client) = &test_suite.health_client {
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
        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(anyhow::anyhow!("Rindexer process exited with status: {}", status));
            }
            Ok(None) => {
                info!("✓ Rindexer process is still running");
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to check Rindexer process status: {}", e));
            }
        }
        
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
        
        // Clean up - kill the Rindexer process
        info!("Cleaning up Rindexer process...");
        let _ = child.kill();
        let _ = child.wait();
        
        info!("✓ Test 8 PASSED: Rindexer started successfully with forked Anvil and health monitoring");
        Ok(())
    }
}
