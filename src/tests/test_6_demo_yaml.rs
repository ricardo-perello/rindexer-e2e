use anyhow::{Result, Context};
use tracing::info;
use crate::test_suite::TestSuite;
use crate::tests::TestCaseImpl;

pub struct DemoYamlTest;

impl TestCaseImpl for DemoYamlTest {
    fn name(&self) -> &str {
        "test_6_demo_yaml"
    }
    
    fn description(&self) -> &str {
        "Test Rindexer with the demo YAML configuration adapted for Anvil"
    }
    
    async fn run(&self, test_suite: &mut TestSuite) -> Result<()> {
        info!("Running Test 6: Demo YAML Test");
        info!("Description: {}", self.description());
        
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
        
        info!("Created Rindexer project with demo YAML at: {:?}", test_suite.project_path);
        
        // Start Rindexer with the demo configuration
        info!("Starting Rindexer with demo configuration...");
        let rindexer = crate::rindexer_client::RindexerInstance::new(&test_suite.rindexer_binary, test_suite.project_path.clone()).await
            .context("Failed to create and start Rindexer instance")?;
        
        test_suite.rindexer = Some(rindexer);
        info!("Rindexer started successfully");
        
        // Wait for Rindexer to start up
        info!("Waiting for Rindexer to be ready...");
        test_suite.wait_for_rindexer_ready(30).await?;
        info!("Rindexer is ready");
        
        // Verify Rindexer is still running
        if !test_suite.is_rindexer_running() {
            return Err(anyhow::anyhow!("Rindexer process is not running"));
        }
        
        info!("✓ Test 6 PASSED: Rindexer started successfully with demo YAML");
        Ok(())
    }
}
