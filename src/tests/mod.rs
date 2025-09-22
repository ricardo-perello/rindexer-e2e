pub mod test_1_basic_connection;
pub mod test_2_contract_discovery;
pub mod test_3_historic_indexing;
pub mod test_4_single_transfer;
pub mod test_5_multiple_transfers;
pub mod test_6_demo_yaml;
pub mod test_8_forked_anvil;

use anyhow::Result;
use crate::test_suite::TestSuite;

pub enum TestCase {
    BasicConnection(test_1_basic_connection::BasicConnectionTest),
    ContractDiscovery(test_2_contract_discovery::ContractDiscoveryTest),
    HistoricIndexing(test_3_historic_indexing::HistoricIndexingTest),
    SingleTransfer(test_4_single_transfer::SingleTransferTest),
    MultipleTransfers(test_5_multiple_transfers::MultipleTransfersTest),
    DemoYaml(test_6_demo_yaml::DemoYamlTest),
    ForkedAnvil(test_8_forked_anvil::ForkedAnvilTest),
}

impl TestCase {
    pub fn name(&self) -> &str {
        match self {
            TestCase::BasicConnection(test) => test.name(),
            TestCase::ContractDiscovery(test) => test.name(),
            TestCase::HistoricIndexing(test) => test.name(),
            TestCase::SingleTransfer(test) => test.name(),
            TestCase::MultipleTransfers(test) => test.name(),
            TestCase::DemoYaml(test) => test.name(),
            TestCase::ForkedAnvil(test) => test.name(),
        }
    }
    
    pub fn description(&self) -> &str {
        match self {
            TestCase::BasicConnection(test) => test.description(),
            TestCase::ContractDiscovery(test) => test.description(),
            TestCase::HistoricIndexing(test) => test.description(),
            TestCase::SingleTransfer(test) => test.description(),
            TestCase::MultipleTransfers(test) => test.description(),
            TestCase::DemoYaml(test) => test.description(),
            TestCase::ForkedAnvil(test) => test.description(),
        }
    }
    
    pub async fn run(&self, test_suite: &mut TestSuite) -> Result<()> {
        match self {
            TestCase::BasicConnection(test) => test.run(test_suite).await,
            TestCase::ContractDiscovery(test) => test.run(test_suite).await,
            TestCase::HistoricIndexing(test) => test.run(test_suite).await,
            TestCase::SingleTransfer(test) => test.run(test_suite).await,
            TestCase::MultipleTransfers(test) => test.run(test_suite).await,
            TestCase::DemoYaml(test) => test.run(test_suite).await,
            TestCase::ForkedAnvil(test) => test.run(test_suite).await,
        }
    }
}

pub trait TestCaseImpl {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn run(&self, test_suite: &mut TestSuite) -> Result<()>;
}

pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
    pub duration: std::time::Duration,
}

pub async fn run_test_suite(rindexer_binary: String, test_names: Option<Vec<String>>) -> Result<Vec<TestResult>> {
    let mut results = Vec::new();
    
    // Define available tests
    let available_tests: Vec<TestCase> = vec![
        TestCase::BasicConnection(test_1_basic_connection::BasicConnectionTest),
        TestCase::ContractDiscovery(test_2_contract_discovery::ContractDiscoveryTest),
        TestCase::HistoricIndexing(test_3_historic_indexing::HistoricIndexingTest),
        TestCase::SingleTransfer(test_4_single_transfer::SingleTransferTest),
        TestCase::MultipleTransfers(test_5_multiple_transfers::MultipleTransfersTest),
        TestCase::DemoYaml(test_6_demo_yaml::DemoYamlTest),
        TestCase::ForkedAnvil(test_8_forked_anvil::ForkedAnvilTest),
    ];
    
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
        
        // Create a scope to ensure TestSuite lives for the entire test
        let result = {
            let mut test_suite = TestSuite::new(rindexer_binary.clone()).await?;
            
            match test.run(&mut test_suite).await {
                Ok(_) => TestResult {
                    name: test.name().to_string(),
                    passed: true,
                    error: None,
                    duration: start_time.elapsed(),
                },
                Err(e) => TestResult {
                    name: test.name().to_string(),
                    passed: false,
                    error: Some(e.to_string()),
                    duration: start_time.elapsed(),
                },
            }
        };
        
        results.push(result);
    }
    
    Ok(results)
}
