use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use anyhow::{Result, Context};
use tracing::info;
use wait_timeout::ChildExt;

pub struct AnvilInstance {
    pub rpc_url: String,
    pub ws_url: String,
    pub process: Option<std::process::Child>,
}

impl AnvilInstance {
    pub async fn start_local(private_key: &str) -> Result<Self> {
        info!("Starting local Anvil instance");
        
        let mut cmd = Command::new("anvil");
        cmd.arg("--chain-id")
           .arg("31337")
           .arg("--accounts")
           .arg("10")
           .arg("--balance")
           .arg("10000")
           .arg("--gas-limit")
           .arg("30000000")
           .arg("--gas-price")
           .arg("1000000000")
           .arg("--block-time")
           .arg("1")
           .arg("--silent")
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let mut child = cmd.spawn()
            .context("Failed to start Anvil")?;
        
        // Wait a bit for Anvil to start
        sleep(Duration::from_millis(2000)).await;
        
        // Check if process is still running
        match child.try_wait()? {
            Some(status) => {
                return Err(anyhow::anyhow!("Anvil exited with status: {}", status));
            }
            None => {
                info!("Anvil process started successfully");
            }
        }
        
        let rpc_url = "http://127.0.0.1:8545".to_string();
        let ws_url = "ws://127.0.0.1:8545".to_string();
        
        // Wait for RPC to be ready
        Self::wait_for_rpc_ready(&rpc_url).await?;
        
        // Fund the test account
        Self::fund_test_account(&rpc_url, private_key).await?;
        
        Ok(Self {
            rpc_url,
            ws_url,
            process: Some(child),
        })
    }
    
    pub async fn connect(rpc_url: String) -> Result<Self> {
        info!("Connecting to existing Anvil instance at: {}", rpc_url);
        
        let ws_url = rpc_url.replace("http://", "ws://");
        
        // Verify connection
        Self::wait_for_rpc_ready(&rpc_url).await?;
        
        Ok(Self {
            rpc_url,
            ws_url,
            process: None,
        })
    }
    
    async fn wait_for_rpc_ready(rpc_url: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 30;
        
        while attempts < MAX_ATTEMPTS {
            match client.post(rpc_url)
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "eth_blockNumber",
                    "params": [],
                    "id": 1
                }))
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        info!("Anvil RPC is ready");
                        return Ok(());
                    }
                }
                Err(_) => {}
            }
            
            attempts += 1;
            sleep(Duration::from_millis(1000)).await;
        }
        
        Err(anyhow::anyhow!("Anvil RPC failed to become ready after {} attempts", MAX_ATTEMPTS))
    }
    
    async fn fund_test_account(_rpc_url: &str, _private_key: &str) -> Result<()> {
        //TODO This would typically fund accounts for testing
        // For now, we'll use the default funded accounts from Anvil
        info!("Using default Anvil funded accounts");
        Ok(())
    }
    
    pub async fn mine_block(&self) -> Result<()> {
        let client = reqwest::Client::new();
        
        let response = client.post(&self.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "evm_mine",
                "params": [],
                "id": 1
            }))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to mine block"));
        }
        
        Ok(())
    }
    
    pub async fn get_block_number(&self) -> Result<u64> {
        let client = reqwest::Client::new();
        
        let response = client.post(&self.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_blockNumber",
                "params": [],
                "id": 1
            }))
            .send()
            .await?;
        
        let result: serde_json::Value = response.json().await?;
        let hex_value = result["result"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?;
        
        let block_number = u64::from_str_radix(hex_value.trim_start_matches("0x"), 16)?;
        Ok(block_number)
    }
}

impl Drop for AnvilInstance {
    fn drop(&mut self) {
        if let Some(mut child) = self.process.take() {
            info!("Shutting down Anvil instance");
            let _ = child.kill();
            let _ = child.wait_timeout(Duration::from_secs(5));
        }
    }
}
