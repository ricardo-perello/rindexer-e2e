use clap::Parser;
use tracing::{info, error};
use tracing_subscriber::{fmt, EnvFilter};

mod test_runner;
mod anvil_setup;
mod rindexer_client;
mod test_flows;

use test_runner::TestRunner;
use anvil_setup::AnvilInstance;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the Rindexer binary
    #[arg(short, long)]
    rindexer_binary: String,
    
    /// Test configuration directory
    #[arg(short, long)]
    config_dir: Option<String>,
    
    /// Anvil RPC URL (if not provided, will start local instance)
    #[arg(long)]
    anvil_url: Option<String>,
    
    /// Anvil private key for funding test accounts
    #[arg(long, default_value = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")]
    private_key: String,
    
    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
    
    /// Use persistent directories instead of temporary ones for debugging
    #[arg(long)]
    persistent_dirs: bool,
    
    /// Directory to store persistent test data (only used with --persistent-dirs)
    #[arg(long, default_value = "./test_output")]
    output_dir: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    // Initialize tracing with configurable log level
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&args.log_level));
    
    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();
    
    info!("Starting Rindexer E2E tests");
    info!("Binary: {}", args.rindexer_binary);
    let config_dir = args.config_dir.unwrap_or_else(|| "test_configs".to_string());
    info!("Config dir: {}", config_dir);
    
    // Setup Anvil instance
    let anvil = if let Some(url) = args.anvil_url {
        info!("Using existing Anvil instance at: {}", url);
        match AnvilInstance::connect(url).await {
            Ok(instance) => instance,
            Err(e) => {
                info!("Failed to connect to existing Anvil instance: {}. Starting local instance...", e);
                AnvilInstance::start_local(&args.private_key).await?
            }
        }
    } else {
        info!("Attempting to connect to existing Anvil instance at http://localhost:8545");
        match AnvilInstance::connect("http://localhost:8545".to_string()).await {
            Ok(instance) => instance,
            Err(e) => {
                info!("Failed to connect to existing Anvil instance: {}. Starting local instance...", e);
                AnvilInstance::start_local(&args.private_key).await?
            }
        }
    };
    
    // Create test runner with persistent directory option
    let mut runner = TestRunner::new(
        &args.rindexer_binary, 
        &config_dir, 
        anvil,
        args.persistent_dirs,
        &args.output_dir
    ).await?;
    
    // Run tests
    match runner.run_all_tests().await {
        Ok(results) => {
            info!("All tests completed");
            for (test_name, result) in results {
                match result {
                    Ok(_) => info!("✓ {}: PASSED", test_name),
                    Err(e) => error!("✗ {}: FAILED - {}", test_name, e),
                }

            }
        }
        Err(e) => {
            error!("Test runner failed: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}

