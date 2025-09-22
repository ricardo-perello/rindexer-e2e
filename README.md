# Rindexer E2E Testing Framework

A comprehensive end-to-end testing framework for [Rindexer](https://rindexer.xyz), the high-performance EVM event indexer built in Rust.

## 🎯 Current Status

**✅ WORKING** - The framework successfully:
- ✅ Connects to Anvil blockchain instances (local and forked)
- ✅ Generates proper Rindexer YAML configurations
- ✅ Starts Rindexer processes and monitors health endpoints
- ✅ Executes comprehensive test suite with proper error handling
- ✅ Tests both local Anvil and forked Ethereum mainnet scenarios

## 🚀 Quick Start

### Prerequisites

1. **Rust** (1.70+)
2. **Anvil** (Foundry's local blockchain)
   ```bash
   curl -L https://foundry.paradigm.xyz | bash
   foundryup
   ```
3. **Rindexer** binary (defaults to `../rindexer/target/release/rindexer_cli`)

### Run Tests

```bash
# Run all tests
cargo run

# Run specific tests
cargo run -- --tests "test_1_basic_connection,test_8_forked_anvil"

# Use custom Rindexer binary
cargo run -- --rindexer-binary /path/to/rindexer_cli

# Debug mode
RUST_LOG=debug cargo run
```

## 📋 Command Line Options

- `--rindexer-binary`: Path to Rindexer binary (default: `../rindexer/target/release/rindexer_cli`)
- `--log-level`: Log level (trace, debug, info, warn, error) (default: `info`)
- `--tests`: Specific tests to run, comma-separated (optional)

## 🏗️ Architecture

The framework provides:

- **Anvil Integration**: Manages local and forked blockchain instances
- **Test Suite**: Orchestrates test execution and Rindexer lifecycle
- **Health Monitoring**: Uses Rindexer's health endpoint for intelligent waiting
- **Configuration Generator**: Creates proper Rindexer YAML configs
- **Event Verification**: Validates indexed events against on-chain data

## 🧪 Available Tests

1. **test_1_basic_connection**: Basic Rindexer connectivity and startup
2. **test_2_contract_discovery**: Contract deployment and event registration
3. **test_3_historic_indexing**: Historic event indexing verification
4. **test_6_demo_yaml**: Demo YAML configuration testing with Anvil
5. **test_8_forked_anvil**: Forked Ethereum mainnet testing with real data

## ⚙️ Test Configuration

Tests generate Rindexer configurations like:

```yaml
name: basic_sync_test
project_type: no-code
networks:
  - name: anvil
    chain_id: 31337
    rpc: http://localhost:8545
storage:
  postgres:
    enabled: false
  csv:
    enabled: true
contracts:
  - name: SimpleERC20
    details:
      - network: anvil
        address: 0x5FbDB2315678afecb367f032d93F642f64180aa3
        start_block: "0"
    abi: ./abis/SimpleERC20.abi.json
    include_events:
      - Transfer
```

## 🛠️ Development

### Adding New Tests

1. Create new test file in `src/tests/`
2. Implement `TestCaseImpl` trait
3. Add test to `TestCase` enum in `src/tests/mod.rs`

### Project Structure

```
src/
├── main.rs              # CLI entry point
├── test_suite.rs        # Test orchestration and lifecycle
├── anvil_setup.rs       # Blockchain instance management
├── rindexer_client.rs   # Rindexer process management
├── health_client.rs     # Health endpoint monitoring
└── tests/               # Test implementations
    ├── mod.rs           # Test case definitions
    ├── test_1_basic_connection.rs
    ├── test_2_contract_discovery.rs
    ├── test_3_historic_indexing.rs
    ├── test_6_demo_yaml.rs
    └── test_8_forked_anvil.rs
```

## 🔍 Features

- **Health Endpoint Integration**: Uses Rindexer's `/health` endpoint for intelligent waiting
- **Forked Testing**: Tests with real Ethereum mainnet data via Anvil fork
- **Comprehensive Logging**: Detailed test execution and debugging information
- **Flexible Configuration**: Easy to add new test cases and scenarios

## 🐛 Troubleshooting

### Common Issues

- **Rindexer not found**: Ensure binary is installed and path is correct
- **Anvil connection failed**: Framework automatically starts Anvil instances
- **Health endpoint timeout**: Check if Rindexer is running and accessible
- **Forked Anvil issues**: Ensure you have internet connection for mainnet fork

### Debug Mode

```bash
RUST_LOG=debug cargo run
```

## 📈 Roadmap

- [x] Basic test suite with health monitoring
- [x] Forked Anvil testing with real data
- [x] Health endpoint integration
- [ ] Contract deployment automation
- [ ] Event generation and verification
- [ ] Database query validation
- [ ] Performance benchmarking
- [ ] CI/CD integration
- [ ] Multi-network testing

## 🤝 Contributing

1. Follow Rust conventions
2. Add tests for new features
3. Update documentation
4. Ensure all tests pass

## 📄 License

Same license as Rindexer project.