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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ContractConfig {
    pub name: String,
    pub details: Vec<ContractDetail>,
    pub abi: Option<String>,
    pub include_events: Option<Vec<EventConfig>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ContractDetail {
    pub network: String,
    pub address: String,
    pub start_block: String,
    pub end_block: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EventConfig {
    pub name: String,
}
use crate::health_client::HealthClient;

/// Shared context for all tests - provides common infrastructure
pub struct TestContext {
    pub anvil: AnvilInstance,
    pub rindexer: Option<RindexerInstance>,
    pub test_contract_address: Option<String>,
    pub temp_dir: Option<TempDir>,
    pub project_path: PathBuf,
    pub rindexer_binary: String,
    pub health_client: Option<HealthClient>,
}

// Keep TestSuite as an alias for backward compatibility during transition
pub type TestSuite = TestContext;

impl TestContext {
    pub async fn new(rindexer_binary: String) -> Result<Self> {
        info!("Setting up fresh test context...");
        
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
    
    /// Deploy a test contract using the Anvil instance
    pub async fn deploy_test_contract(&mut self) -> Result<String> {
        let address = self.anvil.deploy_test_contract().await?;
        self.test_contract_address = Some(address.clone());
        Ok(address)
    }
    
    /// Create a minimal Rindexer configuration
    pub fn create_minimal_config(&self) -> RindexerConfig {
        crate::rindexer_client::RindexerInstance::create_minimal_config(&self.anvil.rpc_url)
    }
    
    /// Create a configuration with a specific contract
    pub fn create_contract_config(&self, contract_address: &str) -> RindexerConfig {
        crate::rindexer_client::RindexerInstance::create_contract_config(&self.anvil.rpc_url, contract_address)
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
        
        // Create Rindexer instance and start indexer
        let mut rindexer = RindexerInstance::new(&self.rindexer_binary, self.project_path.clone());
        
        rindexer.start_indexer().await
            .context("Failed to start Rindexer indexer")?;
        
        self.rindexer = Some(rindexer);
        info!("Rindexer started successfully");
        
        Ok(())
    }
    
    /// Wait for Rindexer sync completion based on log output
    pub async fn wait_for_sync_completion(&mut self, timeout_seconds: u64) -> Result<()> {
        if let Some(rindexer) = &mut self.rindexer {
            rindexer.wait_for_initial_sync_completion(timeout_seconds).await?;
            info!("✓ Rindexer sync completed (detected via logs)");
        }
        Ok(())
    }
    
    /// Wait for Rindexer health endpoint to be ready
    pub async fn wait_for_health_ready(&mut self, timeout_seconds: u64) -> Result<()> {
        if let Some(health_client) = &self.health_client {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            
            match health_client.wait_for_healthy(timeout_seconds).await {
                Ok(_) => info!("✓ Rindexer health endpoint confirms readiness"),
                Err(e) => {
                    warn!("Health endpoint not available, falling back to process check: {}", e);
                    if !self.is_rindexer_running() {
                        return Err(anyhow::anyhow!("Rindexer process is not running"));
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Wait for indexing to complete (health endpoint if available, otherwise logs)
    pub async fn wait_for_indexing_complete(&mut self, timeout_seconds: u64) -> Result<()> {
        if let Some(health_client) = &self.health_client {
            info!("Waiting for indexing to complete using health endpoint...");
            health_client.wait_for_indexing_complete(timeout_seconds).await?;
            info!("✓ Indexing completed according to health endpoint");
        } else {
            self.wait_for_sync_completion(timeout_seconds).await?;
        }
        Ok(())
    }
    
    pub fn get_csv_output_path(&self) -> PathBuf {
        self.project_path.join("generated_csv")
    }

    pub fn is_rindexer_running(&self) -> bool {
        if let Some(rindexer) = &self.rindexer {
            rindexer.is_running()
        } else {
            false
        }
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
