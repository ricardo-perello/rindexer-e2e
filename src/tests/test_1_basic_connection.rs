use anyhow::Result;
use tracing::info;
use crate::test_suite::TestContext;
use crate::tests::Test;

pub struct BasicConnectionTest;

impl Test for BasicConnectionTest {
    fn name(&self) -> &str {
        "test_1_basic_connection"
    }
    
    fn description(&self) -> &str {
        "Test basic Rindexer connection to Anvil with minimal configuration"
    }
    
    // No custom setup needed - uses default (empty)
    
    async fn run(&self, context: &mut TestContext) -> Result<()> {
        info!("Running Test 1: Basic Connection Test");
        info!("Description: {}", self.description());
        
        // Create minimal configuration (just network, no contracts)
        let config = context.create_minimal_config();
        
        // Start Rindexer with minimal config
        context.start_rindexer(config).await?;
        
        // Wait for Rindexer to start up
        context.wait_for_sync_completion(5).await?;
        
        // Verify Rindexer is still running (basic health check)
        if !context.is_rindexer_running() {
            return Err(anyhow::anyhow!("Rindexer process is not running"));
        }
        
        info!("✓ Test 1 PASSED: Rindexer connected successfully with minimal config");
        Ok(())
    }
    
    // No custom teardown needed - uses default (empty)
}
