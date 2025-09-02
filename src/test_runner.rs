use std::collections::HashMap;
use std::path::Path;
use anyhow::{Result, Context};
use tracing::{info, warn};
use serde::{Deserialize, Serialize};

use crate::anvil_setup::AnvilInstance;
use crate::rindexer_client::{RindexerInstance, ContractConfig, ContractDetail};
use crate::test_flows::BasicSyncTest;

#[derive(Debug, Serialize, Deserialize)]
pub struct TestFlow {
    pub name: String,
    pub rindexer_config: RindexerConfig,
    pub test_steps: Vec<TestStep>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RindexerConfig {
    pub name: String,
    pub project_type: String,
    pub config: serde_json::Value,
    pub timestamps: Option<serde_json::Value>,
    pub networks: Vec<NetworkConfig>,
    pub storage: StorageConfig,
    pub native_transfers: NativeTransfersConfig,
    pub contracts: Vec<ContractConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub name: String,
    pub chain_id: u64,
    pub rpc: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageConfig {
    pub postgres: PostgresConfig,
    pub csv: CsvConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostgresConfig {
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CsvConfig {
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NativeTransfersConfig {
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestStep {
    pub name: String,
    pub action: String,
    pub params: Option<serde_json::Value>,
    pub expected_result: Option<String>,
}

pub struct TestRunner {
    rindexer_binary_path: String,
    config_dir: String,
    anvil: AnvilInstance,
}

impl TestRunner {
    pub async fn new(rindexer_binary_path: &str, config_dir: &str, anvil: AnvilInstance) -> Result<Self> {
        Ok(Self {
            rindexer_binary_path: rindexer_binary_path.to_string(),
            config_dir: config_dir.to_string(),
            anvil,
        })
    }
    
    pub async fn run_all_tests(&mut self) -> Result<HashMap<String, Result<()>>> {
        let mut results = HashMap::new();
        
        // Discover test flows
        let test_flows = self.discover_test_flows().await?;
        
        for flow in test_flows {
            info!("Running test flow: {}", flow.name);
            
            let result = self.run_test_flow(&flow).await;
            results.insert(flow.name.clone(), result);
        }
        
        Ok(results)
    }
    
    async fn discover_test_flows(&self) -> Result<Vec<TestFlow>> {
        let mut flows = Vec::new();
        let config_path = Path::new(&self.config_dir);
        
        if !config_path.exists() {
            info!("Config directory doesn't exist, creating basic test flow");
            flows.push(self.create_basic_test_flow().await?);
            return Ok(flows);
        }
        
        // Read all YAML files in the config directory
        for entry in std::fs::read_dir(config_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                info!("Loading test flow from: {:?}", path);
                
                let content = std::fs::read_to_string(&path)?;
                let flow: TestFlow = serde_yaml::from_str(&content)?;
                flows.push(flow);
            }
        }
        
        if flows.is_empty() {
            info!("No test flows found, creating basic test flow");
            flows.push(self.create_basic_test_flow().await?);
        }
        
        Ok(flows)
    }
    
    async fn create_basic_test_flow(&self) -> Result<TestFlow> {
        Ok(TestFlow {
            name: "basic_sync".to_string(),
            rindexer_config: RindexerConfig {
                name: "basic_sync_test".to_string(),
                project_type: "no-code".to_string(),
                config: serde_json::json!({}),
                timestamps: None,
                networks: vec![
                    NetworkConfig {
                        name: "anvil".to_string(),
                        chain_id: 31337,
                        rpc: self.anvil.rpc_url.clone(),
                    }
                ],
                storage: StorageConfig {
                    postgres: PostgresConfig { enabled: false },
                    csv: CsvConfig { enabled: true },
                },
                native_transfers: NativeTransfersConfig { enabled: true },
                contracts: vec![
                    ContractConfig {
                        name: "TestContract".to_string(),
                        details: vec![
                            ContractDetail {
                                network: "anvil".to_string(),
                                address: "0x5FbDB2315678afecb367f032d93F642f64180aa3".to_string(),
                                start_block: "0".to_string(),
                                end_block: None,
                            }
                        ],
                        abi: Some("./abis/ERC20.abi.json".to_string()),
                        include_events: Some(vec!["Transfer".to_string()]),
                    }
                ],
            },
            test_steps: vec![
                TestStep {
                    name: "start_indexing".to_string(),
                    action: "start".to_string(),
                    params: None,
                    expected_result: None,
                },
                TestStep {
                    name: "wait_for_sync".to_string(),
                    action: "wait_sync".to_string(),
                    params: Some(serde_json::json!({"target_block": 10})),
                    expected_result: None,
                },
                TestStep {
                    name: "verify_events".to_string(),
                    action: "verify_events".to_string(),
                    params: None,
                    expected_result: None,
                }
            ],
        })
    }
    
    async fn run_test_flow(&mut self, flow: &TestFlow) -> Result<()> {
        info!("Starting test flow: {}", flow.name);
        
        // Create a temporary Rindexer project directory
        let temp_dir = tempfile::TempDir::new()
            .context("Failed to create temporary directory")?;
        
        let project_path = temp_dir.path().join("test_project");
        std::fs::create_dir(&project_path)?;
        
        // Create abis directory and copy ABI file
        let abis_dir = project_path.join("abis");
        std::fs::create_dir(&abis_dir)?;
        std::fs::copy("abis/ERC20.abi.json", abis_dir.join("ERC20.abi.json"))?;
        
        // Write the Rindexer configuration
        let config_path = project_path.join("rindexer.yaml");
        
        // Write only the Rindexer configuration (test_steps are handled separately)
        let config_content = serde_yaml::to_string(&flow.rindexer_config)?;
        info!("Generated Rindexer config:\n{}", config_content);
        std::fs::write(&config_path, config_content)?;
        
        info!("Created Rindexer project at: {:?}", project_path);
        
        // Start Rindexer from the project directory
        let mut rindexer = RindexerInstance::new(&self.rindexer_binary_path, project_path).await?;
        
        // Execute test steps
        for step in &flow.test_steps {
            info!("Executing step: {}", step.name);
            
            match step.action.as_str() {
                "start" => {
                    // Rindexer is already started in the constructor
                    info!("Rindexer started successfully");
                }
                "wait_sync" => {
                    if let Some(params) = &step.params {
                        if let Some(target_block) = params.get("target_block").and_then(|v| v.as_u64()) {
                            rindexer.wait_for_sync(target_block, 60).await?;
                        }
                    }
                }
                "verify_events" => {
                    // Run the basic sync test verification
                    let basic_test = BasicSyncTest::new(&self.anvil.rpc_url);
                    basic_test.verify_indexed_events().await?;
                }
                _ => {
                    warn!("Unknown test action: {}", step.action);
                }
            }
        }
        
        // Cleanup
        rindexer.stop().await?;
        
        info!("Test flow completed successfully: {}", flow.name);
        Ok(())
    }
}
