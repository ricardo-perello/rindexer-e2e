pub mod anvil_setup;
pub mod health_client;
pub mod rindexer_client;
pub mod test_flows;
pub mod test_runner;
pub mod test_suite;
pub mod tests;

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_basic_sync_test_creation() {
        let test = test_flows::BasicSyncTest::new("http://localhost:8545");
        // Provider is a concrete type, so if we get here without panic, it worked
        assert!(true);
    }
    
    #[test]
    fn test_rindexer_config_serialization() {
        use rindexer_client::{ContractConfig, ContractDetail, EventConfig};
       
        
        let config = ContractConfig {
            name: "TestContract".to_string(),
            details: vec![
                ContractDetail {
                    network: "anvil".to_string(),
                    address: "0x1234...".to_string(),
                    start_block: "0".to_string(),
                    end_block: None,
                }
            ],
            abi: None,
            include_events: Some(vec![EventConfig {
                name: "Transfer".to_string(),
            }]),
        };
        
        
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("name"));
        assert!(yaml.contains("Transfer"));
    }
}
