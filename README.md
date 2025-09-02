# Rindexer E2E Testing Framework

A comprehensive end-to-end testing framework for [Rindexer](https://rindexer.xyz), the high-performance EVM event indexer built in Rust.

## 🎯 Current Status

**✅ WORKING** - The framework successfully:
- ✅ Connects to Anvil blockchain instances
- ✅ Generates proper Rindexer YAML configurations
- ✅ Starts Rindexer processes without parsing errors
- ✅ Executes test flows with proper error handling
- 🔧 **Next**: Deploy test contracts and verify event indexing

## 🚀 Quick Start

### Prerequisites

1. **Rust** (1.70+)
2. **Anvil** (Foundry's local blockchain)
   ```bash
   curl -L https://foundry.paradigm.xyz | bash
   foundryup
   ```
3. **Rindexer** binary
   ```bash
   curl -L https://rindexer.xyz/install.sh | bash
   ```

### Run Tests

```bash
# Start Anvil (in separate terminal)
anvil --chain-id 31337

# Run the test framework
cargo run -- --rindexer-binary ~/.rindexer/bin/rindexer
```

## 📋 Command Line Options

- `--rindexer-binary`: Path to Rindexer binary (required)
- `--config-dir`: Test configuration directory (optional)
- `--anvil-url`: Existing Anvil instance URL (optional)
- `--private-key`: Private key for test accounts (optional)

## 🏗️ Architecture

The framework provides:

- **Anvil Integration**: Manages local blockchain instances
- **Test Runner**: Orchestrates test execution and Rindexer lifecycle
- **Configuration Generator**: Creates proper Rindexer YAML configs
- **Event Verification**: Validates indexed events against on-chain data

## ⚙️ Test Configuration

Tests are defined programmatically and generate Rindexer configurations like:

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
  - name: TestContract
    details:
      - network: anvil
        address: 0x5FbDB2315678afecb367f032d93F642f64180aa3
        start_block: "0"
    abi: ./abis/ERC20.abi.json
    include_events:
      - Transfer
```

## 🧪 Test Flows

### Basic Sync Test

1. **Setup**: Connect to Anvil, create temporary Rindexer project
2. **Configure**: Generate YAML config with ERC20 ABI
3. **Execute**: Start Rindexer and wait for sync
4. **Verify**: Check indexed events against blockchain data
5. **Cleanup**: Stop processes and clean temporary files

## 🛠️ Development

### Adding New Tests

1. Extend `TestFlow` struct in `src/test_runner.rs`
2. Implement test logic in `src/test_flows/`
3. Add new test actions to the runner

### Project Structure

```
src/
├── main.rs              # CLI entry point
├── test_runner.rs       # Test orchestration
├── anvil_setup.rs       # Blockchain instance management
├── rindexer_client.rs   # Rindexer process management
└── test_flows/          # Test implementations
    ├── basic_sync.rs    # Basic sync verification
    └── mod.rs
```

## 🔍 Current Implementation

The framework successfully:
- Parses Rindexer configurations correctly
- Handles ABI file requirements
- Manages temporary project directories
- Provides proper error messages and logging

**Next Steps**:
1. Deploy test ERC20 contracts to Anvil
2. Generate Transfer events for indexing
3. Verify Rindexer database output
4. Add comprehensive event verification

## 🐛 Troubleshooting

### Common Issues

- **Rindexer not found**: Ensure binary is installed and path is correct
- **Anvil connection failed**: Check if Anvil is running on port 8545
- **ABI file missing**: Framework automatically copies required ABI files

### Debug Mode

```bash
RUST_LOG=debug cargo run -- --rindexer-binary ~/.rindexer/bin/rindexer
```

## 📈 Roadmap

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