use anyhow::{Result, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, debug};

#[derive(Debug, Deserialize, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub services: HealthServices,
    pub indexing: Option<IndexingStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HealthServices {
    pub database: String,
    pub indexing: String,
    pub sync: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IndexingStatus {
    pub active_tasks: u32,
    pub is_running: bool,
}

pub struct HealthClient {
    client: Client,
    base_url: String,
}

impl HealthClient {
    pub fn new(port: u16) -> Self {
        Self {
            client: Client::new(),
            base_url: format!("http://localhost:{}", port),
        }
    }

    pub async fn get_health(&self) -> Result<HealthResponse> {
        let url = format!("{}/health", self.base_url);
        debug!("Checking health at: {}", url);
        
        let response = self.client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .context("Failed to send health request")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Health endpoint returned status: {}", response.status()));
        }

        let health: HealthResponse = response
            .json()
            .await
            .context("Failed to parse health response")?;

        Ok(health)
    }

    pub async fn wait_for_healthy(&self, timeout_seconds: u64) -> Result<()> {
        info!("Waiting for Rindexer health endpoint to be healthy (timeout: {}s)", timeout_seconds);
        
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_seconds);
        
        while start_time.elapsed() < timeout {
            match self.get_health().await {
                Ok(health) => {
                    debug!("Health status: {:?}", health);
                    
                    // Check if all services are healthy
                    if health.status == "healthy" && 
                       health.services.database == "healthy" &&
                       health.services.indexing == "healthy" {
                        info!("✓ Rindexer is healthy and ready");
                        return Ok(());
                    }
                    
                    // If indexing is not running but other services are healthy, 
                    // it might mean indexing is complete
                    if health.status == "healthy" && 
                       health.services.database == "healthy" &&
                       health.services.sync == "healthy" {
                        if let Some(indexing) = &health.indexing {
                            if !indexing.is_running && indexing.active_tasks == 0 {
                                info!("✓ Rindexer indexing completed (no active tasks)");
                                return Ok(());
                            }
                        } else {
                            // No indexing status means indexing might be complete
                            info!("✓ Rindexer appears to be ready (no indexing status)");
                            return Ok(());
                        }
                    }
                }
                Err(e) => {
                    debug!("Health check failed: {}, retrying...", e);
                }
            }
            
            // Wait before next check
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        
        Err(anyhow::anyhow!("Health check timeout after {}s", timeout_seconds))
    }

    pub async fn wait_for_indexing_complete(&self, timeout_seconds: u64) -> Result<()> {
        info!("Waiting for Rindexer indexing to complete (timeout: {}s)", timeout_seconds);
        
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_seconds);
        
        while start_time.elapsed() < timeout {
            match self.get_health().await {
                Ok(health) => {
                    debug!("Health status: {:?}", health);
                    
                    // Check if indexing is complete
                    if let Some(indexing) = &health.indexing {
                        if !indexing.is_running && indexing.active_tasks == 0 {
                            info!("✓ Rindexer indexing completed (no active tasks)");
                            return Ok(());
                        }
                    } else {
                        // No indexing status might mean indexing is complete
                        if health.status == "healthy" && health.services.sync == "healthy" {
                            info!("✓ Rindexer indexing appears complete (no indexing status)");
                            return Ok(());
                        }
                    }
                }
                Err(e) => {
                    debug!("Health check failed: {}, retrying...", e);
                }
            }
            
            // Wait before next check
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
        
        Err(anyhow::anyhow!("Indexing completion timeout after {}s", timeout_seconds))
    }

    pub async fn is_healthy(&self) -> bool {
        match self.get_health().await {
            Ok(health) => health.status == "healthy",
            Err(_) => false,
        }
    }
}
