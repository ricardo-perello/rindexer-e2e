use anyhow::{Result, Context};
use tracing::{info, warn};
use std::path::PathBuf;
use tempfile::TempDir;

use crate::anvil_setup::AnvilInstance;
use crate::rindexer_client::RindexerInstance;
// Config structs for Rindexer
#[derive(Debug, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct NetworkConfig {
    pub name: String,
    pub chain_id: u64,
    pub rpc: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StorageConfig {
    pub postgres: PostgresConfig,
    pub csv: CsvConfig,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PostgresConfig {
    pub enabled: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CsvConfig {
    pub enabled: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct NativeTransfersConfig {
    pub enabled: bool,
}
use crate::rindexer_client::{ContractConfig, ContractDetail, EventConfig};
use crate::health_client::HealthClient;

pub struct TestSuite {
    pub anvil: AnvilInstance,
    pub rindexer: Option<RindexerInstance>,
    pub test_contract_address: Option<String>,
    pub temp_dir: Option<TempDir>,
    pub project_path: PathBuf,
    pub rindexer_binary: String,
    pub health_client: Option<HealthClient>,
}

impl TestSuite {
    pub async fn new(rindexer_binary: String) -> Result<Self> {
        info!("Setting up fresh test suite...");
        
        // Kill any existing Anvil processes and start fresh
        info!("Killing any existing Anvil processes...");
        let _ = std::process::Command::new("pkill")
            .arg("-f")
            .arg("anvil")
            .output();
        
        // Wait for processes to be killed and port to be free
        wait_for_port_free(8545, 10).await?;
        
        // Start a fresh Anvil instance
        let anvil = AnvilInstance::start_local("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80").await
            .context("Failed to start Anvil instance")?;
        
        info!("Anvil ready at: {}", anvil.rpc_url);
        
        // Create temporary directory for this test run
        let temp_dir = TempDir::new()
            .context("Failed to create temporary directory")?;
        
        let project_path = temp_dir.path().join("test_project");
        std::fs::create_dir(&project_path)
            .context("Failed to create project directory")?;
        
        Ok(Self {
            anvil,
            rindexer: None,
            test_contract_address: None,
            temp_dir: Some(temp_dir),
            project_path,
            rindexer_binary,
            health_client: Some(HealthClient::new(8080)), // Default health port
        })
    }
    
    pub async fn cleanup(&mut self) -> Result<()> {
        info!("Cleaning up test suite...");
        
        // Stop Rindexer if running
        if let Some(mut rindexer) = self.rindexer.take() {
            if let Err(e) = rindexer.stop().await {
                warn!("Error stopping Rindexer: {}", e);
            }
        }
        
        // Anvil will be cleaned up automatically when the process is dropped
        
        // TempDir will be cleaned up automatically on drop
        self.temp_dir.take();
        
        info!("Test suite cleanup completed");
        Ok(())
    }
    
    pub async fn deploy_test_contract(&mut self) -> Result<String> {
        info!("Deploying test contract...");
        
        // Deploy the SimpleERC20 contract
        let output = std::process::Command::new("forge")
            .args(&[
                "create",
                "--rpc-url", &self.anvil.rpc_url,
                "--private-key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
                "--broadcast",
                "contracts/SimpleERC20.sol:SimpleERC20"
            ])
            .output()
            .context("Failed to run forge command")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Contract deployment failed: {}", stderr));
        }
        
        // Parse the contract address from forge output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let address_line = stdout.lines()
            .find(|line| line.contains("Deployed to:"))
            .ok_or_else(|| anyhow::anyhow!("Could not find contract address in forge output"))?;
        
        let address = address_line.split_whitespace()
            .last()
            .ok_or_else(|| anyhow::anyhow!("Could not parse contract address"))?;
        
        self.test_contract_address = Some(address.to_string());
        info!("Test contract deployed at: {}", address);
        
        Ok(address.to_string())
    }
    
    pub fn create_minimal_config(&self) -> RindexerConfig {
        RindexerConfig {
            name: "minimal_test".to_string(),
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
            native_transfers: NativeTransfersConfig { enabled: false },
            contracts: vec![],
        }
    }
    
    pub fn create_contract_config(&self, contract_address: &str) -> RindexerConfig {
        let mut config = self.create_minimal_config();
        config.name = "contract_test".to_string();
        config.contracts = vec![
            ContractConfig {
                name: "SimpleERC20".to_string(),
                details: vec![
                    ContractDetail {
                        network: "anvil".to_string(),
                        address: contract_address.to_string(),
                        start_block: "0".to_string(),
                        end_block: None,
                    }
                ],
                abi: Some("./abis/SimpleERC20.abi.json".to_string()),
                include_events: Some(vec![EventConfig { name: "Transfer".to_string() }]),
            }
        ];
        config
    }
    
    pub async fn start_rindexer(&mut self, config: RindexerConfig) -> Result<()> {
        // Create abis directory and copy ABI file
        let abis_dir = self.project_path.join("abis");
        std::fs::create_dir(&abis_dir)
            .context("Failed to create abis directory")?;
        
        std::fs::copy("abis/SimpleERC20.abi.json", abis_dir.join("SimpleERC20.abi.json"))
            .context("Failed to copy ABI file")?;
        
        // Write the Rindexer configuration
        let config_path = self.project_path.join("rindexer.yaml");
        let config_yaml = serde_yaml::to_string(&config)
            .context("Failed to serialize config to YAML")?;
        
        std::fs::write(&config_path, config_yaml)
            .context("Failed to write config file")?;
        
        info!("Created Rindexer project at: {:?}", self.project_path);
        
        // Start Rindexer (the new method already starts the process)
        let rindexer = RindexerInstance::new(&self.rindexer_binary, self.project_path.clone()).await
            .context("Failed to create and start Rindexer instance")?;
        
        self.rindexer = Some(rindexer);
        info!("Rindexer started successfully");
        
        Ok(())
    }
    
    pub async fn wait_for_rindexer_ready(&mut self, timeout_seconds: u64) -> Result<()> {
        // First, wait for Rindexer to start up
        if let Some(rindexer) = &mut self.rindexer {
            rindexer.wait_for_initial_sync_completion(timeout_seconds).await?;
        }
        
        // Then use health endpoint to verify it's ready
        if let Some(health_client) = &self.health_client {
            // Give health endpoint a moment to become available
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            
            match health_client.wait_for_healthy(timeout_seconds).await {
                Ok(_) => {
                    info!("✓ Rindexer health endpoint confirms readiness");
                }
                Err(e) => {
                    warn!("Health endpoint not available, falling back to process check: {}", e);
                    // Fallback to process check if health endpoint is not available
                    if !self.is_rindexer_running() {
                        return Err(anyhow::anyhow!("Rindexer process is not running"));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    pub fn get_csv_output_path(&self) -> PathBuf {
        self.project_path.join("generated_csv")
    }
    
    pub async fn wait_for_indexing_complete(&mut self, timeout_seconds: u64) -> Result<()> {
        if let Some(health_client) = &self.health_client {
            info!("Waiting for indexing to complete using health endpoint...");
            health_client.wait_for_indexing_complete(timeout_seconds).await?;
            info!("✓ Indexing completed according to health endpoint");
        } else {
            // Fallback to log-based detection
            if let Some(rindexer) = &mut self.rindexer {
                rindexer.wait_for_initial_sync_completion(timeout_seconds).await?;
            }
        }
        Ok(())
    }

    pub fn is_rindexer_running(&self) -> bool {
        if let Some(rindexer) = &self.rindexer {
            if let Some(_process) = &rindexer.process {
                // Process exists, assume it's running
                // Note: We can't call try_wait() here because it requires &mut
                // The process will be checked properly in the RindexerInstance methods
                return true;
            }
        }
        false
    }
}

async fn wait_for_port_free(port: u16, max_attempts: u32) -> Result<()> {
    for attempt in 1..=max_attempts {
        // Try to connect to the port - if it fails, the port is free
        match tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port)).await {
            Ok(_) => {
                // Port is still in use, wait a bit
                if attempt < max_attempts {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
            Err(_) => {
                // Port is free, we can proceed
                return Ok(());
            }
        }
    }
    Err(anyhow::anyhow!("Port {} is still in use after {} attempts", port, max_attempts))
}
