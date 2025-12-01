# Robonomics Zombienet Integration Tests

This directory contains integration tests for the Robonomics network using [Zombienet](https://github.com/paritytech/zombienet).

## Overview

The integration test suite spawns a temporary test network consisting of:
- A Rococo local relay chain with 2 validators (Alice and Bob)
- A Robonomics parachain with 2 collators

The tests verify:
1. **Network Initialization** - Ensures relay chain and parachain nodes start correctly
2. **Block Production** - Verifies blocks are being produced on both chains
3. **Extrinsic Submission** - Tests basic transaction submission and inclusion

## Prerequisites

### System Requirements
- Linux or macOS (zombienet native provider)
- Node.js >= 16.x
- npm or yarn
- Rust toolchain (for building Robonomics)
- 4GB+ RAM
- ~10GB free disk space

### Required Tools

1. **Rust & Cargo** (for building Robonomics)
   ```bash
   curl https://sh.rustup.rs -sSf | sh
   ```

2. **Node.js** (for test scripts)
   ```bash
   # Using nvm
   curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
   nvm install 18
   
   # Or using system package manager
   sudo apt install nodejs npm  # Ubuntu/Debian
   brew install node            # macOS
   ```

3. **Zombienet & Polkadot** (automatically downloaded by test runner)
   - The test runner will automatically download zombienet and polkadot binaries
   - Alternatively, you can manually download them:
     ```bash
     # Zombienet
     curl -L -o zombienet https://github.com/paritytech/zombienet/releases/download/v1.3.106/zombienet-linux-x64
     chmod +x zombienet
     
     # Polkadot
     curl -L -o polkadot https://github.com/paritytech/polkadot-sdk/releases/download/polkadot-v1.15.2/polkadot
     chmod +x polkadot
     ```

## Quick Start

### Running Tests Locally

1. **Build Robonomics** (if not already built)
   ```bash
   cd /path/to/robonomics
   cargo build --release
   ```

2. **Run the test suite**
   ```bash
   cd scripts/zombienet
   ./run-tests.sh
   ```

The script will:
- Download required binaries (zombienet, polkadot) if not present
- Install Node.js test dependencies
- Spawn the test network
- Execute integration tests
- Clean up automatically

### Running Tests with Pre-built Binaries

If you already have the environment set up:

```bash
./run-tests.sh --skip-setup
```

## Test Configuration

### Network Configuration

The network is defined in `robonomics-local.toml`:

```toml
[relaychain]
default_command = "polkadot"
chain = "rococo-local"

  [[relaychain.nodes]]
  name = "alice"
  validator = true

  [[relaychain.nodes]]
  name = "bob"
  validator = true

[[parachains]]
id = 2000
chain = "dev"

  [parachains.collator]
  name = "collator-01"
  command = "robonomics"

  [[parachains.collators]]
  name = "collator-02"
  command = "robonomics"
```

### Test Configuration

Test parameters can be modified in `tests/integration-tests.js`:

```javascript
const TESTS_CONFIG = {
  relayWsUrl: 'ws://127.0.0.1:9944',           // Relay chain WS endpoint
  parachainWsUrl: 'ws://127.0.0.1:9988',       // Parachain WS endpoint
  timeout: 300000,                              // Global timeout (5 min)
  blockProductionWaitTime: 60000,               // Wait time for blocks (1 min)
};
```

## Test Suite Details

### 1. Network Initialization Test
- Connects to relay chain and parachain nodes
- Verifies chain names and RPC endpoints are accessible
- Ensures both chains are running

### 2. Block Production Test
- Records initial block numbers on both chains
- Waits for a specified duration
- Verifies block numbers have increased
- Ensures continuous block production

### 3. Extrinsic Submission Test
- Uses Alice's development account
- Submits a `system.remark` extrinsic to the parachain
- Waits for transaction inclusion in a block
- Verifies successful execution

## CI/CD Integration

### GitHub Actions

The tests can be integrated into GitHub Actions workflows. Example:

```yaml
name: Zombienet Tests

on:
  pull_request:
    branches: [master, release/*]
  push:
    branches: [master]

jobs:
  zombienet-tests:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4
      
      - name: Install Rust toolchain
        uses: actions-rust-lang/setup-rust-toolchain@v1
      
      - name: Install system dependencies
        run: |
          sudo apt update
          sudo apt install -y pkg-config protobuf-compiler
      
      - name: Build Robonomics
        run: cargo build --release
      
      - name: Run Zombienet tests
        run: ./scripts/zombienet/run-tests.sh
```

## Directory Structure

```
scripts/zombienet/
├── README.md                      # This file
├── run-tests.sh                   # Main test runner script
├── robonomics-local.toml          # Zombienet network configuration
├── bin/                           # Downloaded binaries (auto-created)
│   ├── zombienet
│   └── polkadot
└── tests/
    ├── package.json               # Node.js dependencies
    └── integration-tests.js       # Test suite implementation
```

## Troubleshooting

### Common Issues

1. **Port conflicts**
   - Error: `Address already in use`
   - Solution: Ensure ports 9944, 9988, 30333, etc. are free
   - Check with: `lsof -i :9944` or `netstat -tunlp | grep 9944`

2. **Binary not found**
   - Error: `robonomics binary not found`
   - Solution: Build Robonomics first: `cargo build --release`

3. **Node.js version**
   - Error: Module compatibility issues
   - Solution: Use Node.js >= 16.x

4. **Memory issues**
   - Error: Process killed or OOM
   - Solution: Ensure at least 4GB free RAM

5. **Timeout errors**
   - Error: `Transaction timeout` or `Connection timeout`
   - Solution: Increase timeout values in test configuration

### Debug Mode

To see detailed logs from the network:

1. Enable verbose logging in `robonomics-local.toml`:
   ```toml
   default_args = ["-lparachain=trace"]
   ```

2. Check zombienet logs in the temporary directory (printed during execution)

### Manual Testing

You can manually spawn the network without running tests:

```bash
cd scripts/zombienet
./bin/zombienet spawn robonomics-local.toml --provider native
```

This keeps the network running for manual interaction.

## Development

### Adding New Tests

To add new test cases, edit `tests/integration-tests.js`:

```javascript
async function testNewFeature() {
  testResults.total++;
  log.test('Testing new feature...');
  
  try {
    const api = await connectToNode(TESTS_CONFIG.parachainWsUrl, 'Parachain');
    
    // Your test logic here
    
    await api.disconnect();
    
    log.success('New feature test passed');
    testResults.passed++;
    return true;
  } catch (error) {
    log.error(`New feature test failed: ${error.message}`);
    testResults.failed++;
    return false;
  }
}

// Add to runTests() function:
await testNewFeature();
```

### Modifying Network Configuration

Edit `robonomics-local.toml` to:
- Add more validators/collators
- Change chain specifications
- Adjust logging levels
- Configure additional parachains

## Resources

- [Zombienet Documentation](https://paritytech.github.io/zombienet/)
- [Polkadot.js API Documentation](https://polkadot.js.org/docs/)
- [Robonomics Documentation](https://wiki.robonomics.network)
- [Substrate Development](https://docs.substrate.io/)

## Support

For issues or questions:
- GitHub Issues: https://github.com/airalab/robonomics/issues
- Matrix: https://matrix.to/#/#robonomics:matrix.org

## License

Apache-2.0
