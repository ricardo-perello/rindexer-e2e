use clap::Parser;
use tracing::{info, error};
use tracing_subscriber::{fmt, EnvFilter};

mod anvil_setup;
mod rindexer_client;
mod test_suite;
mod tests;
mod health_client;

use tests::run_test_suite;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the Rindexer binary
    #[arg(short, long, default_value = "../rindexer/target/release/rindexer_cli")]
    rindexer_binary: String,
    
    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
    
    /// Specific tests to run (comma-separated). If not provided, runs all tests.
    #[arg(long)]
    tests: Option<String>,
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
    
    info!("Starting Rindexer E2E Test Suite");
    info!("Binary: {}", args.rindexer_binary);
    
    // Run the test suite (it manages its own Anvil instances)
    let test_names = args.tests.map(|t| t.split(',').map(|s| s.trim().to_string()).collect());
    
    match run_test_suite(args.rindexer_binary, test_names).await {
        Ok(results) => {
            info!("Test suite completed");
            let mut passed = 0;
            let mut failed = 0;
            
            for result in results {
                if result.passed {
                    info!("✓ {}: PASSED ({:.2}s)", result.name, result.duration.as_secs_f64());
                    passed += 1;
                } else {
                    error!("✗ {}: FAILED ({:.2}s) - {}", result.name, result.duration.as_secs_f64(), result.error.unwrap_or_default());
                    failed += 1;
                }
            }
            
            info!("Test Results: {} passed, {} failed", passed, failed);
            
            if failed > 0 {
                std::process::exit(1);
            }
        }
        Err(e) => {
            error!("Test suite failed: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}

