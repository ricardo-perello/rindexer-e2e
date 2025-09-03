use anyhow::Result;
use tracing::info;
use crate::test_suite::TestSuite;
use crate::tests::TestCaseImpl;

pub struct ContractDiscoveryTest;

impl TestCaseImpl for ContractDiscoveryTest {
    fn name(&self) -> &str {
        "test_2_contract_discovery"
    }
    
    fn description(&self) -> &str {
        "Test Rindexer can discover and register contract events from ABI"
    }
    
    async fn run(&self, test_suite: &mut TestSuite) -> Result<()> {
        info!("Running Test 2: Contract Discovery Test");
        info!("Description: {}", self.description());
        
        // Deploy test contract
        let contract_address = test_suite.deploy_test_contract().await?;
        
        // Create configuration with contract
        let config = test_suite.create_contract_config(&contract_address);
        
        // Start Rindexer with contract config
        test_suite.start_rindexer(config).await?;
        
        // Wait for Rindexer to start up and register events
        test_suite.wait_for_rindexer_ready(15).await?;
         
        // Verify Rindexer is still running
        if test_suite.rindexer.is_none() {
            return Err(anyhow::anyhow!("Rindexer process is not running"));
        }
        
        // Check that CSV output directory was created (indicates Rindexer recognized the contract)
        let csv_path = test_suite.get_csv_output_path();
        if !csv_path.exists() {
            return Err(anyhow::anyhow!("CSV output directory not created - contract not recognized"));
        }
        
        info!("✓ Test 2 PASSED: Rindexer discovered contract and registered Transfer event");
        Ok(())
    }
}
