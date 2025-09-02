use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use anyhow::{Result, Context};
use tracing::info;
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use wait_timeout::ChildExt;

#[derive(Debug)]
pub struct RindexerInstance {
    pub process: Option<std::process::Child>,
    pub config_path: String,
    pub temp_dir: Option<TempDir>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RindexerConfig {
    pub rpc_url: String,
    pub ws_url: String,
    pub database_url: String,
    pub contracts: Vec<ContractConfig>,
    pub start_block: Option<u64>,
    pub end_block: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractConfig {
    pub name: String,
    pub details: Vec<ContractDetail>,
    pub abi: Option<String>,
    pub include_events: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDetail {
    pub network: String,
    pub address: String,
    pub start_block: String,
    pub end_block: Option<String>,
}



impl RindexerInstance {
    pub async fn new(binary_path: &str, project_path: std::path::PathBuf) -> Result<Self> {
        info!("Starting Rindexer instance from project: {:?}", project_path);
        
        // Start Rindexer process from the project directory
        let mut cmd = Command::new(binary_path);
        cmd.current_dir(&project_path)
           .arg("start")
           .arg("all")
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let mut child = cmd.spawn()
            .context("Failed to start Rindexer")?;
        
        // Wait for Rindexer to start
        sleep(Duration::from_millis(3000)).await;
        
        // Check if process is still running
        match child.try_wait()? {
            Some(status) => {
                // Try to read stderr to get the error message
                if let Some(mut stderr) = child.stderr.take() {
                    use std::io::Read;
                    let mut stderr_output = String::new();
                    let _ = stderr.read_to_string(&mut stderr_output);
                    if !stderr_output.is_empty() {
                        return Err(anyhow::anyhow!("Rindexer exited with status: {}. Error output: {}", status, stderr_output));
                    }
                }
                return Err(anyhow::anyhow!("Rindexer exited with status: {}", status));
            }
            None => {
                info!("Rindexer process started successfully");
            }
        }
        
        Ok(Self {
            process: Some(child),
            config_path: project_path.to_string_lossy().to_string(),
            temp_dir: None,
        })
    }
    
    pub async fn wait_for_sync(&mut self, target_block: u64, timeout_seconds: u64) -> Result<()> {
        info!("Waiting for Rindexer to sync to block {}", target_block);
        
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_seconds);
        
        while start_time.elapsed() < timeout {
                    // Check if process is still running
        if let Some(process) = &mut self.process {
            match process.try_wait()? {
                Some(status) => {
                    return Err(anyhow::anyhow!("Rindexer process exited with status: {}", status));
                }
                None => {}
            }
        }
            
            // Here you would typically check the database or API to see current sync status
            // For now, we'll just wait and assume it's working
            sleep(Duration::from_millis(1000)).await;
            
            if start_time.elapsed() >= timeout {
                return Err(anyhow::anyhow!("Timeout waiting for sync to block {}", target_block));
            }
        }
        
        info!("Rindexer sync completed to block {}", target_block);
        Ok(())
    }
    
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.take() {
            info!("Stopping Rindexer instance");
            let _ = child.kill();
            let _ = child.wait_timeout(Duration::from_secs(5));
        }
        
        if let Some(temp_dir) = self.temp_dir.take() {
            let _ = temp_dir.close();
        }
        
        Ok(())
    }
    
    pub async fn restart(&mut self, binary_path: &str) -> Result<()> {
        info!("Restarting Rindexer instance");
        
        self.stop().await?;
        
        // Read existing config
        let config_content = std::fs::read_to_string(&self.config_path)?;
        let _config: RindexerConfig = serde_yaml::from_str(&config_content)?;
        
        // Create new temporary directory
        let temp_dir = TempDir::new()
            .context("Failed to create temporary directory")?;
        
        let config_path = temp_dir.path().join("config.yaml");
        std::fs::write(&config_path, config_content)?;
        
        // Start new process
        let mut cmd = Command::new(binary_path);
        cmd.arg("--config")
           .arg(&config_path)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let child = cmd.spawn()
            .context("Failed to restart Rindexer")?;
        
        // Wait for startup
        sleep(Duration::from_millis(3000)).await;
        
        self.process = Some(child);
        self.config_path = config_path.to_string_lossy().to_string();
        self.temp_dir = Some(temp_dir);
        
        Ok(())
    }
}

impl Drop for RindexerInstance {
    fn drop(&mut self) {
        if let Some(mut child) = self.process.take() {
            info!("Shutting down Rindexer instance");
            let _ = child.kill();
            let _ = child.wait_timeout(Duration::from_secs(5));
        }
        
        if let Some(temp_dir) = self.temp_dir.take() {
            let _ = temp_dir.close();
        }
    }
}
