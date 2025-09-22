use std::time::Duration;
use std::process::Stdio;
use tokio::time::sleep;
use anyhow::{Result, Context};
use tracing::{info, debug, error};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

pub struct AnvilInstance {
    pub rpc_url: String,
    pub ws_url: String,
    pub process: Option<tokio::process::Child>,
}

impl AnvilInstance {
    pub async fn start_forked() -> Result<Self> {
        info!("Starting Anvil forked from Ethereum mainnet");
        
        let mut cmd = TokioCommand::new("anvil");
        cmd.arg("--fork-url")
           .arg("https://eth-mainnet.g.alchemy.com/v2/JQceHZ-KHeV8btdy7ACh_")
           .arg("--chain-id")
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
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let mut child = cmd.spawn()
            .context("Failed to start forked Anvil")?;
        
        // Start log streaming for Anvil
        Self::start_log_streaming(&mut child).await;
        
        // Wait a bit for Anvil to start
        sleep(Duration::from_millis(2000)).await;
        
        // Check if process is still running
        match child.try_wait()? {
            Some(status) => {
                return Err(anyhow::anyhow!("Forked Anvil exited with status: {}", status));
            }
            None => {
                info!("Forked Anvil process started successfully");
            }
        }
        
        // Wait for RPC to be ready
        Self::wait_for_rpc_ready("http://127.0.0.1:8545").await?;
        
        Ok(Self {
            process: Some(child),
            rpc_url: "http://127.0.0.1:8545".to_string(),
            ws_url: "ws://127.0.0.1:8545".to_string(),
        })
    }

    pub async fn start_local(private_key: &str) -> Result<Self> {
        info!("Starting local Anvil instance");
        
        let mut cmd = TokioCommand::new("anvil");
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
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let mut child = cmd.spawn()
            .context("Failed to start Anvil")?;
        
        // Start log streaming for Anvil
        Self::start_log_streaming(&mut child).await;
        
        // Wait a bit for Anvil to start
        sleep(Duration::from_millis(500)).await;
        
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
            sleep(Duration::from_millis(200)).await;
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
    
    async fn start_log_streaming(child: &mut tokio::process::Child) {
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            tokio::spawn(async move {
                while let Ok(Some(line)) = lines.next_line().await {
                    debug!("[ANVIL] {}", line);
                }
            });
        }
        
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            
            tokio::spawn(async move {
                while let Ok(Some(line)) = lines.next_line().await {
                    error!("[ANVIL ERROR] {}", line);
                }
            });
        }
    }
}

impl Drop for AnvilInstance {
    fn drop(&mut self) {
        if let Some(mut child) = self.process.take() {
            info!("Shutting down Anvil instance");
            let _ = child.kill();
            // Note: tokio::process::Child doesn't have wait_timeout, 
            // but the process will be cleaned up when the child is dropped
        }
    }
}
