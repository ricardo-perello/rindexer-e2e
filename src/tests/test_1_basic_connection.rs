use anyhow::Result;
use tracing::info;
use crate::test_suite::TestSuite;
use crate::tests::TestCaseImpl;

pub struct BasicConnectionTest;

impl TestCaseImpl for BasicConnectionTest {
    fn name(&self) -> &str {
        "test_1_basic_connection"
    }
    
    fn description(&self) -> &str {
        "Test basic Rindexer connection to Anvil with minimal configuration"
    }
    
    async fn run(&self, test_suite: &mut TestSuite) -> Result<()> {
        info!("Running Test 1: Basic Connection Test");
        info!("Description: {}", self.description());
        
        // Create minimal configuration (just network, no contracts)
        let config = test_suite.create_minimal_config();
        
        // Start Rindexer with minimal config
        test_suite.start_rindexer(config).await?;
        
        // Wait for Rindexer to start up
        test_suite.wait_for_rindexer_ready(10).await?;
        
        // Verify Rindexer is still running (basic health check)
        if test_suite.rindexer.is_none() {
            return Err(anyhow::anyhow!("Rindexer process is not running"));
        }
        
        info!("✓ Test 1 PASSED: Rindexer connected successfully with minimal config");
        Ok(())
    }
}
