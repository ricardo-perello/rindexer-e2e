pub mod test_1_basic_connection;
pub mod test_2_contract_discovery;
pub mod test_3_historic_indexing;
pub mod test_6_demo_yaml;
pub mod test_8_forked_anvil;

use anyhow::Result;
use crate::test_suite::TestContext;

/// Standard test trait following Setup → Test → Teardown pattern
pub trait Test {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    
    /// Optional setup phase - uses default if not implemented
    async fn setup(&self, context: &mut TestContext) -> Result<()> { 
        Ok(()) 
    }
    
    /// The actual test logic
    async fn run(&self, context: &mut TestContext) -> Result<()>;
    
    /// Optional teardown phase - uses default if not implemented  
    async fn teardown(&self, context: &mut TestContext) -> Result<()> { 
        Ok(()) 
    }
}

/// Wrapper enum for all test types
pub enum TestCase {
    BasicConnection(test_1_basic_connection::BasicConnectionTest),
    ContractDiscovery(test_2_contract_discovery::ContractDiscoveryTest),
    HistoricIndexing(test_3_historic_indexing::HistoricIndexingTest),
    DemoYaml(test_6_demo_yaml::DemoYamlTest),
    ForkedAnvil(test_8_forked_anvil::ForkedAnvilTest),
}

impl Test for TestCase {
    fn name(&self) -> &str {
        match self {
            TestCase::BasicConnection(test) => test.name(),
            TestCase::ContractDiscovery(test) => test.name(),
            TestCase::HistoricIndexing(test) => test.name(),
            TestCase::DemoYaml(test) => test.name(),
            TestCase::ForkedAnvil(test) => test.name(),
        }
    }
    
    fn description(&self) -> &str {
        match self {
            TestCase::BasicConnection(test) => test.description(),
            TestCase::ContractDiscovery(test) => test.description(),
            TestCase::HistoricIndexing(test) => test.description(),
            TestCase::DemoYaml(test) => test.description(),
            TestCase::ForkedAnvil(test) => test.description(),
        }
    }
    
    async fn setup(&self, context: &mut TestContext) -> Result<()> {
        match self {
            TestCase::BasicConnection(test) => test.setup(context).await,
            TestCase::ContractDiscovery(test) => test.setup(context).await,
            TestCase::HistoricIndexing(test) => test.setup(context).await,
            TestCase::DemoYaml(test) => test.setup(context).await,
            TestCase::ForkedAnvil(test) => test.setup(context).await,
        }
    }
    
    async fn run(&self, context: &mut TestContext) -> Result<()> {
        match self {
            TestCase::BasicConnection(test) => test.run(context).await,
            TestCase::ContractDiscovery(test) => test.run(context).await,
            TestCase::HistoricIndexing(test) => test.run(context).await,
            TestCase::DemoYaml(test) => test.run(context).await,
            TestCase::ForkedAnvil(test) => test.run(context).await,
        }
    }
    
    async fn teardown(&self, context: &mut TestContext) -> Result<()> {
        match self {
            TestCase::BasicConnection(test) => test.teardown(context).await,
            TestCase::ContractDiscovery(test) => test.teardown(context).await,
            TestCase::HistoricIndexing(test) => test.teardown(context).await,
            TestCase::DemoYaml(test) => test.teardown(context).await,
            TestCase::ForkedAnvil(test) => test.teardown(context).await,
        }
    }
}

pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
    pub duration: std::time::Duration,
}

/// Get all available tests
pub fn get_available_tests() -> Vec<TestCase> {
    vec![
        TestCase::BasicConnection(test_1_basic_connection::BasicConnectionTest),
        TestCase::ContractDiscovery(test_2_contract_discovery::ContractDiscoveryTest),
        TestCase::HistoricIndexing(test_3_historic_indexing::HistoricIndexingTest),
        TestCase::DemoYaml(test_6_demo_yaml::DemoYamlTest),
        TestCase::ForkedAnvil(test_8_forked_anvil::ForkedAnvilTest),
    ]
}

/// Run all tests with proper Setup → Test → Teardown lifecycle
pub async fn run_tests(rindexer_binary: String, test_names: Option<Vec<String>>) -> Result<Vec<TestResult>> {
    let mut results = Vec::new();
    
    // Get available tests
    let available_tests = get_available_tests();
    
    // Filter tests if specific names provided
    let tests_to_run = if let Some(names) = test_names {
        available_tests.into_iter()
            .filter(|test| names.contains(&test.name().to_string()))
            .collect()
    } else {
        available_tests
    };
    
    for test in tests_to_run {
        let start_time = std::time::Instant::now();
        
        // Create test context for this test
        let mut context = TestContext::new(rindexer_binary.clone()).await?;
        
        let result = {
            // Run the full test lifecycle: Setup → Test → Teardown
            match test.setup(&mut context).await {
                Ok(_) => {
                    match test.run(&mut context).await {
                        Ok(_) => {
                            // Always run teardown, even if test passed
                            let _ = test.teardown(&mut context).await;
                            TestResult {
                                name: test.name().to_string(),
                                passed: true,
                                error: None,
                                duration: start_time.elapsed(),
                            }
                        }
                        Err(e) => {
                            // Run teardown even if test failed
                            let _ = test.teardown(&mut context).await;
                            TestResult {
                                name: test.name().to_string(),
                                passed: false,
                                error: Some(e.to_string()),
                                duration: start_time.elapsed(),
                            }
                        }
                    }
                }
                Err(e) => {
                    // Run teardown even if setup failed
                    let _ = test.teardown(&mut context).await;
                    TestResult {
                        name: test.name().to_string(),
                        passed: false,
                        error: Some(format!("Setup failed: {}", e)),
                        duration: start_time.elapsed(),
                    }
                }
            }
        };
        
        // Cleanup context
        let _ = context.cleanup().await;
        
        results.push(result);
    }
    
    Ok(results)
}
