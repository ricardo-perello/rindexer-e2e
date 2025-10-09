use anyhow::Result;
use tracing::info;
use std::pin::Pin;
use std::future::Future;

use crate::test_suite::TestContext;
use crate::tests::registry::{TestDefinition, TestModule};

pub struct HealthAssertionsTests;

impl TestModule for HealthAssertionsTests {
    fn get_tests() -> Vec<TestDefinition> {
        vec![
            TestDefinition::new(
                "test_health_endpoint_ready_and_complete",
                "Assert /health shows ready and indexing tasks go to 0",
                health_endpoint_ready_and_complete_test,
            ).with_timeout(120),
        ]
    }
}

fn health_endpoint_ready_and_complete_test(context: &mut TestContext) -> Pin<Box<dyn Future<Output = Result<()>> + '_>> {
    Box::pin(async move {
        info!("Running Health Endpoint Assertions Test");

        // Use contract config to ensure at least one task runs, and bound the range
        let contract_address = context.deploy_test_contract().await?;
        let mut config = context.create_contract_config(&contract_address);
        // Bound indexing so health can report completion even with auto-mined blocks
        let current_block = context.anvil.get_block_number().await?;
        if let Some(contract) = config.contracts.get_mut(0) {
            if let Some(detail) = contract.details.get_mut(0) {
                detail.end_block = Some(current_block.to_string());
            }
        }
        context.start_rindexer(config).await?;

        // Wait for health endpoint to report readiness
        context.wait_for_health_ready(10).await?;

        // While indexing, /health should be available; then ensure completion
        // Use log-based completion to avoid racing the health server shutdown after bounded sync
        context.wait_for_sync_completion(30).await?;

        info!("✓ Health Endpoint Assertions Test PASSED: ready and indexing completed");
        Ok(())
    })
}


