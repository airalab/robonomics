# Robonet - Robonomics Local Network Tool

Robonet is a comprehensive CLI tool for spawning local Robonomics networks and running integration tests. Built on top of [ZombieNet SDK](https://github.com/paritytech/zombienet-sdk), it provides an easy way to test Robonomics functionality locally without needing external infrastructure.

## Features

- **Multiple Network Topologies**: Support for simple (single parachain) and complex (with AssetHub) network configurations
- **Comprehensive Testing**: Built-in integration tests for XCM, CPS, Claim pallets, and more
- **Developer-Friendly CLI**: Clean command-line interface with progress indicators and colored output
- **CI/CD Ready**: JSON output format and proper exit codes for automation
- **No Configuration Files**: All network configuration is hardcoded in Rust for simplicity

## Installation

### Build from Source

```bash
# From the repository root
cargo build --release -p robonet

# The binary will be available at
./target/release/robonet
```

### Using Nix

```bash
# Development shell with robonet available
nix develop .#localnet

# Run directly
nix develop .#localnet --command robonet --help
```

## Quick Start

### Spawn a Simple Network

```bash
# Spawn a simple network (relay + robonomics parachain)
robonet spawn --topology simple

# Network stays running, press Ctrl+C to stop
```

### Spawn Network with AssetHub

```bash
# Spawn network with AssetHub for XCM testing
robonet spawn --topology with-assethub
```

### Run Integration Tests

```bash
# Run all tests (spawns network automatically)
robonet test

# Run specific tests
robonet test -t xcm_upward -t xcm_downward

# Run tests on already running network
robonet test --no-spawn

# Run tests with fail-fast
robonet test --fail-fast

# Output results as JSON for CI
robonet test --format json
```

## Network Topologies

### Simple Topology

The simple topology includes:
- **Relay Chain**: rococo-local with 2 validators (alice, bob)
- **Robonomics Parachain** (para_id: 2000): 2 collators
  - collator-1: `ws://127.0.0.1:9988`
  - collator-2: `ws://127.0.0.1:9989`
- **Relay RPC**: `ws://127.0.0.1:9944`

Use this topology for:
- Basic functionality testing
- Parachain-specific feature testing
- CPS and Claim pallet tests

### With-AssetHub Topology

The with-assethub topology includes:
- **Relay Chain**: rococo-local with 2 validators (alice, bob)
- **AssetHub Parachain** (para_id: 1000): `ws://127.0.0.1:9910`
- **Robonomics Parachain** (para_id: 2000): `ws://127.0.0.1:9988`
- **HRMP Channels**: Bidirectional channels between AssetHub and Robonomics
- **Relay RPC**: `ws://127.0.0.1:9944`

Use this topology for:
- XCM message testing (upward, downward, lateral)
- Token teleport tests
- Cross-chain communication tests

## Available Tests

### Network Tests
- **network_initialization**: Verifies all nodes are connectable
- **block_production**: Checks that blocks are being produced

### Basic Functionality
- **extrinsic_submission**: Tests basic transaction submission

### XCM Tests
- **xcm_upward_message**: Tests messages from parachain to relay chain
- **xcm_downward_message**: Tests messages from relay chain to parachain
- **xcm_token_teleport**: Tests token transfers between parachains (requires AssetHub)

### Pallet Tests
- **cps_pallet**: Tests Cyber-Physical Systems pallet functionality
- **claim_pallet**: Tests Claim pallet functionality

## CLI Reference

### Global Options

```
-v, --verbose        Verbose output (-v, -vv, -vvv for increasing verbosity)
-f, --format FORMAT  Output format: text (default) or json
```

### Commands

#### `robonet spawn`

Spawn a local network.

```bash
robonet spawn [OPTIONS]

Options:
  --topology TOPOLOGY  Network topology [default: simple] [possible values: simple, with-assethub]
  --persist            Keep network running (default: waits for Ctrl+C)
  --timeout SECONDS    Network spawn timeout [default: 300]
```

**Examples:**

```bash
# Spawn simple network
robonet spawn

# Spawn with AssetHub, custom timeout
robonet spawn --topology with-assethub --timeout 600

# Spawn without waiting
robonet spawn --persist=false
```

#### `robonet test`

Run integration tests.

```bash
robonet test [OPTIONS]

Options:
  --topology TOPOLOGY   Network topology [default: with-assethub]
  --fail-fast           Stop on first test failure
  -t, --test TEST       Specific test(s) to run (can be specified multiple times)
  --timeout SECONDS     Network spawn timeout [default: 60]
  --no-spawn            Skip network spawning
```

**Examples:**

```bash
# Run all tests
robonet test

# Run specific tests
robonet test -t network_initialization -t block_production

# Run XCM tests only
robonet test -t xcm_upward -t xcm_downward -t xcm_teleport

# Run on existing network
robonet spawn --topology with-assethub &
sleep 30
robonet test --no-spawn

# CI-friendly output
robonet test --format json > test-results.json
```

## Test Guidelines

### Adding a New Integration Test

1. **Define the Test Function**

Add your test function to `tools/robonet/src/tests.rs`:

```rust
/// Test: My new feature
async fn test_my_new_feature(topology: &NetworkTopology) -> Result<()> {
    // Get appropriate endpoints based on topology
    let endpoints = match topology {
        NetworkTopology::Simple => NetworkEndpoints::simple(),
        NetworkTopology::WithAssethub => NetworkEndpoints::with_assethub(),
    };
    
    // Connect to the parachain
    let client = OnlineClient::<PolkadotConfig>::from_url(&endpoints.collator_1_ws)
        .await
        .context("Failed to connect to parachain")?;
    
    // Your test logic here
    log::debug!("Testing my new feature");
    
    // Use dev accounts for signing
    let alice = dev::alice();
    
    // Create and submit extrinsic
    let call = subxt::dynamic::tx(
        "MyPallet",
        "my_extrinsic",
        vec![/* parameters */],
    );
    
    let mut progress = client
        .tx()
        .sign_and_submit_then_watch_default(&call, &alice)
        .await
        .context("Failed to submit transaction")?;
    
    // Wait for in block
    while let Some(status) = progress.next().await {
        let status = status.context("Failed to get transaction status")?;
        if let Some(in_block) = status.as_in_block() {
            log::debug!("Transaction included in block: {:?}", in_block);
            break;
        }
    }
    
    Ok(())
}
```

2. **Register the Test**

Add your test to the `run_integration_tests` function:

```rust
// In run_integration_tests function
if test_filter.is_none() || test_filter.as_ref().unwrap().iter().any(|f| "my_feature".contains(f.as_str())) {
    results.push(run_test("test_my_new_feature", || test_my_new_feature(topology)).await);
    if fail_fast && results.last().unwrap().status == TestStatus::Failed {
        log::warn!("Stopping test execution due to failure (fail-fast mode)");
        return build_results(results, suite_start, json_output);
    }
}
```

3. **Test Your Integration**

```bash
# Run just your test
robonet test -t my_feature -v

# Run with network debugging
RUST_LOG=debug robonet test -t my_feature
```

### Test Best Practices

1. **Use Descriptive Names**: Test names should clearly describe what they're testing
2. **Handle Topology**: Consider whether your test needs AssetHub or can run on simple topology
3. **Add Context to Errors**: Use `.context()` to provide meaningful error messages
4. **Log Progress**: Use `log::debug!` and `log::info!` for debugging
5. **Clean Up**: Tests run sequentially on the same network, ensure state doesn't leak
6. **Fast Feedback**: Keep tests focused and fast; avoid unnecessary waiting

### Common Test Patterns

#### Testing Pallet Extrinsics

```rust
let client = OnlineClient::<PolkadotConfig>::from_url(&endpoint).await?;
let signer = dev::alice();

let call = subxt::dynamic::tx("Pallet", "extrinsic", vec![]);
let mut progress = client.tx().sign_and_submit_then_watch_default(&call, &signer).await?;

while let Some(status) = progress.next().await {
    if let Some(in_block) = status?.as_in_block() {
        // Transaction included
        break;
    }
}
```

#### Testing Storage Queries

```rust
let storage_query = subxt::dynamic::storage("Pallet", "StorageItem", vec![]);
let result = client.storage().at_latest().await?.fetch(&storage_query).await?;
```

#### Testing Events

```rust
let mut progress = client.tx().sign_and_submit_then_watch_default(&call, &signer).await?;

while let Some(status) = progress.next().await {
    if let Some(in_block) = status?.as_in_block() {
        let events = in_block.fetch_events().await?;
        
        for event in events.iter() {
            let event = event?;
            if event.pallet_name() == "MyPallet" && event.variant_name() == "MyEvent" {
                // Event found!
            }
        }
        break;
    }
}
```

## CI Integration

### GitHub Actions

```yaml
- name: Build robonet
  run: cargo build --release -p robonet

- name: Run integration tests
  run: |
    ./target/release/robonet test \
      --format json \
      --fail-fast \
      > test-results.json
  
- name: Upload test results
  if: always()
  uses: actions/upload-artifact@v3
  with:
    name: test-results
    path: test-results.json
```

### Exit Codes

- `0`: Success - all tests passed
- `1`: Tests failed - one or more tests failed
- `2`: Network spawn failed - could not start the network
- `3`: Timeout - operation timed out
- `4`: Invalid arguments - command-line arguments were invalid

### JSON Output Format

```json
{
  "total": 9,
  "passed": 8,
  "failed": 1,
  "skipped": 0,
  "duration": 45.3,
  "tests": [
    {
      "name": "network_initialization",
      "status": "passed",
      "duration": 2.1
    },
    {
      "name": "xcm_upward_message",
      "status": "failed",
      "duration": 5.2,
      "error": "Connection timeout"
    }
  ]
}
```

## Troubleshooting

### Network Fails to Spawn

```bash
# Increase timeout
robonet spawn --timeout 600

# Check for port conflicts
lsof -i :9944  # Relay chain
lsof -i :9988  # Robonomics
lsof -i :9910  # AssetHub

# Enable verbose logging
RUST_LOG=debug robonet spawn -vvv
```

### Tests Fail to Connect

```bash
# Ensure network is running
robonet spawn &
sleep 30  # Wait for stabilization

# Run tests with --no-spawn
robonet test --no-spawn -vvv
```

### Binary Not Found

```bash
# Make sure you built the correct package
cargo build --release -p robonet

# Binary location
ls -la target/release/robonet
```

## Development

### Project Structure

```
tools/robonet/
├── Cargo.toml           # Package configuration
├── build.rs             # Uses robonomics-runtime-subxt-api
└── src/
    ├── main.rs          # CLI entry point
    ├── cli.rs           # Command-line argument parsing
    ├── network.rs       # Network configuration and spawning
    ├── logging.rs       # Logging setup
    ├── health.rs        # Health check utilities
    └── tests/
        ├── mod.rs       # Test runner and infrastructure
        ├── network.rs   # ✅ Network tests (fully implemented)
        ├── xcm.rs       # ✅ XCM tests (fully implemented)
        ├── cps.rs       # ✅ CPS tests (fully implemented)
        └── claim.rs     # ✅ Claim tests (fully implemented)
```

### Architecture

Robonet uses the following Robonomics crates:

- **`robonomics-runtime-subxt-api`**: Type-safe runtime API generated from metadata
  - Located at `runtime/robonomics/subxt-api`
  - Extracts metadata at build time from the runtime
  - Generates compile-time verified API via subxt
  - See [subxt-api README](../../runtime/robonomics/subxt-api/README.md) for details

- **`libcps`**: CPS pallet interaction library for test implementation
  - Located at `tools/libcps`
  - Also uses `robonomics-runtime-subxt-api` for blockchain interactions

### Running Tests

```bash
# Check compilation
cargo check -p robonet

# Build
cargo build -p robonet

# Run with verbose logging
RUST_LOG=debug ./target/debug/robonet spawn -vvv
```

## License

Apache-2.0

## Contributing

Contributions are welcome! Please ensure:
1. Tests pass locally
2. Code follows existing style
3. New tests include documentation
4. Commit messages are descriptive
