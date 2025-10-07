use anyhow::{Result, Context};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::time::interval;
use tracing::{info, debug, warn};
use alloy::{
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
    network::EthereumWallet,
};
use alloy::rpc::types::TransactionRequest;

pub struct LiveFeeder {
    anvil_url: String,
    private_key: String,
    contract_address: Option<Address>,
    tx_interval: Duration,
    mine_interval: Duration,
    stop_tx: Option<mpsc::UnboundedSender<()>>,
}

impl LiveFeeder {
    pub fn new(anvil_url: String, private_key: String) -> Self {
        Self {
            anvil_url,
            private_key,
            contract_address: None,
            tx_interval: Duration::from_secs(2), // Submit tx every 2 seconds
            mine_interval: Duration::from_secs(1), // Mine block every 1 second
            stop_tx: None,
        }
    }

    pub fn with_contract(mut self, contract_address: Address) -> Self {
        self.contract_address = Some(contract_address);
        self
    }

    pub fn with_tx_interval(mut self, interval: Duration) -> Self {
        self.tx_interval = interval;
        self
    }

    pub fn with_mine_interval(mut self, interval: Duration) -> Self {
        self.mine_interval = interval;
        self
    }

    /// Start the live feeder in the background
    pub async fn start(&mut self) -> Result<()> {
        let (stop_tx, stop_rx) = mpsc::unbounded_channel();
        self.stop_tx = Some(stop_tx);

        let anvil_url = self.anvil_url.clone();
        let private_key = self.private_key.clone();
        let contract_address = self.contract_address;
        let tx_interval = self.tx_interval;
        let mine_interval = self.mine_interval;

        info!("Starting live feeder with tx_interval={:?}, mine_interval={:?}", tx_interval, mine_interval);

        // Use Arc<Mutex<Option<UnboundedReceiver>>> to share the receiver
        let stop_rx = Arc::new(Mutex::new(Some(stop_rx)));

        // Spawn transaction submission task
        let tx_task = {
            let anvil_url = anvil_url.clone();
            let stop_rx = stop_rx.clone();
            tokio::spawn(async move {
                let mut tx_timer = interval(tx_interval);
                let mut tx_counter = 0u64;

                loop {
                    tokio::select! {
                        _ = tx_timer.tick() => {
                            if let Err(e) = Self::submit_test_transaction(&anvil_url, &private_key, contract_address, tx_counter).await {
                                warn!("Failed to submit transaction {}: {}", tx_counter, e);
                            } else {
                                debug!("Submitted transaction {}", tx_counter);
                                tx_counter += 1;
                            }
                        }
                        _ = async {
                            if let Some(mut rx) = stop_rx.lock().await.take() {
                                let _ = rx.recv().await;
                            }
                        } => {
                            info!("Transaction feeder stopped");
                            break;
                        }
                    }
                }
            })
        };

        // Spawn mining task
        let mine_task = {
            let anvil_url = anvil_url.clone();
            let stop_rx = stop_rx.clone();
            tokio::spawn(async move {
                let mut mine_timer = interval(mine_interval);
                let mut block_counter = 0u64;

                loop {
                    tokio::select! {
                        _ = mine_timer.tick() => {
                            if let Err(e) = Self::mine_block(&anvil_url).await {
                                warn!("Failed to mine block {}: {}", block_counter, e);
                            } else {
                                debug!("Mined block {}", block_counter);
                                block_counter += 1;
                            }
                        }
                        _ = async {
                            if let Some(mut rx) = stop_rx.lock().await.take() {
                                let _ = rx.recv().await;
                            }
                        } => {
                            info!("Mining feeder stopped");
                            break;
                        }
                    }
                }
            })
        };

        // Wait for both tasks to complete (they'll run until stopped)
        tokio::select! {
            _ = tx_task => {},
            _ = mine_task => {},
        }

        Ok(())
    }

    /// Stop the live feeder
    pub fn stop(&self) {
        if let Some(stop_tx) = &self.stop_tx {
            let _ = stop_tx.send(());
        }
    }

    async fn submit_test_transaction(
        anvil_url: &str,
        private_key: &str,
        contract_address: Option<Address>,
        tx_counter: u64,
    ) -> Result<()> {
        let signer: PrivateKeySigner = private_key.parse()
            .context("Invalid private key")?;
        let wallet = EthereumWallet::from(signer);

        let provider = ProviderBuilder::new()
            .wallet(wallet)
            .on_http(anvil_url.parse()?);

        // Create a simple ETH transfer or contract interaction
        let tx_request = if let Some(contract_addr) = contract_address {
            // Contract interaction - call setNumber with tx_counter
            let call_data = Self::encode_set_number_call(tx_counter);
            TransactionRequest::default()
                .to(contract_addr)
                .input(call_data.into())
        } else {
            // Simple ETH transfer to a random address
            let recipient = Self::generate_test_address(tx_counter);
            TransactionRequest::default()
                .to(recipient)
                .value(U256::from(1000000000000000u64)) // 0.001 ETH
        };

        let pending_tx = provider
            .send_transaction(tx_request)
            .await
            .context("Failed to send transaction")?;

        debug!("Transaction submitted: {:?}", pending_tx.tx_hash());
        Ok(())
    }

    async fn mine_block(anvil_url: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let mine_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "anvil_mine",
            "params": [1],
            "id": 1
        });

        let response = client
            .post(anvil_url)
            .header("Content-Type", "application/json")
            .json(&mine_request)
            .send()
            .await
            .context("Failed to send mine request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to mine block: HTTP {} - {}", status, body);
        }

        Ok(())
    }

    fn encode_set_number_call(value: u64) -> Vec<u8> {
        // Simple ABI encoding for setNumber(uint256) - this is a simplified version
        // In a real implementation, you'd use proper ABI encoding
        let mut data = vec![0x3f, 0xb5, 0xc1, 0xcb]; // setNumber(uint256) function selector
        let mut value_bytes = [0u8; 32];
        value_bytes.copy_from_slice(&U256::from(value).to_be_bytes::<32>());
        data.extend_from_slice(&value_bytes);
        data
    }

    fn generate_test_address(tx_counter: u64) -> Address {
        // Generate a deterministic test address based on tx_counter
        let mut bytes = [0u8; 20];
        bytes[0] = 0x42; // Prefix to make it look like a real address
        bytes[1..8].copy_from_slice(&tx_counter.to_be_bytes()[..7]);
        Address::from(bytes)
    }
}

impl Drop for LiveFeeder {
    fn drop(&mut self) {
        self.stop();
    }
}
