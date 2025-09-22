use anyhow::Result;
use tracing::info;
use std::fs;
use crate::test_suite::TestContext;
use crate::tests::Test;

pub struct HistoricIndexingTest;

impl Test for HistoricIndexingTest {
    fn name(&self) -> &str {
        "test_3_historic_indexing"
    }
    
    fn description(&self) -> &str {
        "Test Rindexer can index historic events from contract deployment"
    }
    
    async fn run(&self, context: &mut TestContext) -> Result<()> {
        info!("Running Test 3: Historic Indexing Test");
        info!("Description: {}", self.description());
        
        // Deploy test contract (this creates a Transfer event)
        let contract_address = context.deploy_test_contract().await?;
        
        // Create configuration with contract
        let config = context.create_contract_config(&contract_address);
        
        // Start Rindexer with contract config
        context.start_rindexer(config).await?;
        
        // Wait for Rindexer to complete historic indexing using health endpoint
        context.wait_for_sync_completion(20).await?;
        
        // Verify CSV file was created and contains the deployment Transfer event
        let csv_path = context.get_csv_output_path().join("SimpleERC20").join("simpleerc20-transfer.csv");
        
        if !csv_path.exists() {
            return Err(anyhow::anyhow!("Transfer CSV file not found"));
        }
        
        // Read and verify CSV content
        let csv_content = fs::read_to_string(&csv_path)?;
        let lines: Vec<&str> = csv_content.lines().collect();
        
        if lines.len() < 2 {
            return Err(anyhow::anyhow!("CSV file should have at least header + 1 data row"));
        }
        
        // Check that we have the deployment Transfer event (from 0x0 to deployer)
        let data_line = lines[1]; // Skip header
        if !data_line.contains(&contract_address.to_lowercase()) {
            return Err(anyhow::anyhow!("CSV does not contain expected contract address"));
        }
        
        if !data_line.contains("0x0000000000000000000000000000000000000000") {
            return Err(anyhow::anyhow!("CSV does not contain expected zero address (minting)"));
        }
        
        info!("✓ Test 3 PASSED: Historic Transfer event indexed correctly");
        info!("CSV contains {} lines", lines.len());
        
        Ok(())
    }
}
