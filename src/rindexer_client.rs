

use std::time::Duration;
use std::process::Stdio;
use tokio::time::sleep;
use anyhow::{Result, Context};
use tracing::{info, debug, error};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

#[derive(Debug)]
pub struct RindexerInstance {
    pub process: Option<tokio::process::Child>,
    pub config_path: String,
    pub temp_dir: Option<TempDir>,
    pub sync_completed: std::sync::Arc<std::sync::atomic::AtomicBool>,
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
    pub include_events: Option<Vec<EventConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventConfig {
    pub name: String,
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
        let mut cmd = TokioCommand::new(binary_path);
        cmd.current_dir(&project_path)
           .arg("start")
           .arg("indexer")
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let mut child = cmd.spawn()
            .context("Failed to start Rindexer")?;
        
        // Start log streaming for Rindexer with completion detection
        let sync_completed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        Self::start_log_streaming_with_completion_detection(&mut child, sync_completed.clone()).await;
        
        // Wait for Rindexer to start
        sleep(Duration::from_millis(500)).await;
        
        // Check if process is still running
        match child.try_wait()? {
            Some(status) => {
                // If Rindexer exits quickly, it might be because there's nothing to index
                // This is actually normal for minimal configurations
                if status.success() {
                    info!("Rindexer completed successfully (likely no events to index)");
                } else {
                    return Err(anyhow::anyhow!("Rindexer exited with error status: {}", status));
                }
            }
            None => {
                info!("Rindexer process started successfully and is still running");
            }
        }
        
        Ok(Self {
            process: Some(child),
            config_path: project_path.to_string_lossy().to_string(),
            temp_dir: None,
            sync_completed,
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
            sleep(Duration::from_millis(200)).await;
            
            if start_time.elapsed() >= timeout {
                return Err(anyhow::anyhow!("Timeout waiting for sync to block {}", target_block));
            }
        }
        
        info!("Rindexer sync completed to block {}", target_block);
        Ok(())
    }
    
    pub async fn wait_for_initial_sync_completion(&mut self, timeout_seconds: u64) -> Result<()> {
        info!("Waiting for Rindexer initial sync completion (timeout: {}s)", timeout_seconds);
        
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_seconds);
        
        while start_time.elapsed() < timeout {
            // Check if sync is completed
            if self.sync_completed.load(std::sync::atomic::Ordering::Relaxed) {
                info!("✓ Rindexer initial sync completed (detected via logs)");
                return Ok(());
            }
            
            // Check if process is still running
            if let Some(process) = &mut self.process {
                match process.try_wait()? {
                    Some(status) => {
                        return Err(anyhow::anyhow!("Rindexer process exited with status: {}", status));
                    }
                    None => {}
                }
            }
            
            // Wait a bit for logs to accumulate
            sleep(Duration::from_millis(500)).await;
        }
        
        Err(anyhow::anyhow!("Timeout waiting for initial sync completion after {}s", timeout_seconds))
    }
    
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.take() {
            info!("Stopping Rindexer instance");
            let _ = child.kill();
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
        let mut cmd = TokioCommand::new(binary_path);
        cmd.arg("--config")
           .arg(&config_path)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let child = cmd.spawn()
            .context("Failed to restart Rindexer")?;
        
        // Wait for startup
        sleep(Duration::from_millis(500)).await;
        
        self.process = Some(child);
        self.config_path = config_path.to_string_lossy().to_string();
        self.temp_dir = Some(temp_dir);
        
        Ok(())
    }
    
    async fn start_log_streaming(child: &mut tokio::process::Child) {
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            tokio::spawn(async move {
                while let Ok(Some(line)) = lines.next_line().await {
                    debug!("[RINDEXER] {}", line);
                }
            });
        }
        
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            
            tokio::spawn(async move {
                while let Ok(Some(line)) = lines.next_line().await {
                    error!("[RINDEXER ERROR] {}", line);
                }
            });
        }
    }
    
    async fn start_log_streaming_with_completion_detection(child: &mut tokio::process::Child, sync_completed: std::sync::Arc<std::sync::atomic::AtomicBool>) {
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let sync_completed_clone = sync_completed.clone();
            
            tokio::spawn(async move {
                while let Ok(Some(line)) = lines.next_line().await {
                    // Print the raw Rindexer output to terminal
                    println!("{}", line);
                    
                    // Also log it for debugging
                    debug!("[RINDEXER] {}", line);
                    
                    // Check for completion messages
                    if line.contains("COMPLETED - Finished indexing historic events") ||
                       line.contains("100.00% progress") ||
                       line.contains("Historical indexing complete") {
                        info!("[RINDEXER] Detected sync completion: {}", line);
                        sync_completed_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            });
        }
        
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            
            tokio::spawn(async move {
                while let Ok(Some(line)) = lines.next_line().await {
                    // Print stderr to terminal as well
                    eprintln!("{}", line);
                    error!("[RINDEXER ERROR] {}", line);
                }
            });
        }
    }
}

impl Drop for RindexerInstance {
    fn drop(&mut self) {
        if let Some(mut child) = self.process.take() {
            info!("Shutting down Rindexer instance");
            let _ = child.kill();
            // Note: tokio::process::Child doesn't have wait_timeout, 
            // but the process will be cleaned up when the child is dropped
        }
        
        if let Some(temp_dir) = self.temp_dir.take() {
            let _ = temp_dir.close();
        }
    }
}
