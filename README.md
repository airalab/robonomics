# Robonomics

[![Web3 Foundation Grants — Wave Two Recipient](https://github.com/airalab/robonomics/blob/master/web3_foundation_grants_badge_black.jpg)](https://medium.com/web3foundation/web3-foundation-grants-wave-two-recipients-16d9b996501d)

[![License](https://img.shields.io/github/license/airalab/robonomics)](https://github.com/airalab/robonomics/blob/master/LICENSE)
[![Release](https://img.shields.io/github/release/airalab/robonomics.svg)](https://github.com/airalab/robonomics/releases)
[![Nightly](https://github.com/airalab/robonomics/workflows/Nightly/badge.svg)](https://github.com/airalab/robonomics/actions/workflows/nightly.yml)
[![Downloads](https://img.shields.io/github/downloads/airalab/robonomics/total.svg)](https://github.com/airalab/robonomics/releases)
[![Matrix](https://img.shields.io/matrix/robonomics:matrix.org)](https://matrix.to/#/#robonomics:matrix.org)

> Robonomics implementation in Rust based on the [Polkadot SDK](https://polkadot.com/platform/sdk/). For more specific guides, like how to be a node, see the [Robonomics Wiki](https://wiki.robonomics.network).

Robonomics platform includes a set of open-source packages and infrastructure for Robotics, Smart Cities and Industry 4.0 developers.

## Quick Start

The fastest way to get started with Robonomics is using Nix flakes to run directly from GitHub, or use pre-built binaries.

### Option 1: Run Directly with Nix (Easiest & Recommended)

No downloads, no builds, no setup - just run! Nix will automatically fetch and cache the binary.

1. Install Nix with flakes support (one-time setup):

```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

2. Run Robonomics directly from GitHub:

```bash
# Run the latest version
nix run github:airalab/robonomics

# Run in development mode
nix run github:airalab/robonomics -- --dev

# Run a specific version
nix run github:airalab/robonomics/v4.1.0 -- --dev
```

3. Open [Polkadot.js Apps](https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:9944) to interact with your local node

That's it! Nix handles everything - downloads, dependencies, and caching. Works on **Linux, macOS, and Windows (WSL)**.

### Option 2: Using Pre-built Binaries

1. Download the latest release:

```bash
# Visit https://get.robonomics.network
# Or download directly from releases
wget https://github.com/airalab/robonomics/releases/download/v4.1.0/robonomics
chmod +x robonomics
```

2. Run a local development node:

```bash
./robonomics --dev
```

3. Open [Polkadot.js Apps](https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:9944) to interact with your local node

## Try it out

Once you have a node running, you can:

- [Open Polkadot.js Apps](https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:9944) - Connect to your local node
- [Connect to Robonomics Network](https://polkadot.js.org/apps/?rpc=wss://kusama.rpc.robonomics.network/) - Connect to the live network
- Explore the [Robonomics Wiki](https://wiki.robonomics.network) for tutorials
- Join our [Matrix community](https://matrix.to/#/#robonomics:matrix.org) for support

## Repository Structure

This repository is organized as a Cargo workspace with the following structure:

### Node Binary

- **`bin/robonomics/`** - Main binary implementation
  - The Robonomics Network Omni Node with CLI interface
  - Built using `polkadot-omni-node-lib` for maximum compatibility

### Runtime

- **`runtime/robonomics/`** - Robonomics parachain runtime
  - WASM runtime implementation for the Robonomics Network
  - Includes configurations for Kusama and Polkadot relay chains
  - Integrates all custom pallets and standard Substrate pallets

### Custom Pallets

- **`frame/`** - Custom FRAME pallets for IoT and robotics
  - `datalog/` - Immutable on-chain data logging with time-series storage
  - `digital-twin/` - Digital twin state management and topic-based data organization
  - `launch/` - Robot/device launch commands with parameter support
  - `liability/` - Smart contract-like agreements for robotics tasks
  - `rws/` - Robonomics Web Services (RWS) subscription management
  - `xcm-info/` - XCM integration utilities
  - `wrapped-asset/` - Native token wrapping functionality

### Chain Specifications

- **`chains/`** - Chain specification files for different networks

### Tools

- **`tools/libcps/`** - Robonomics CPS (Cyber-Physical Systems) library and CLI
  - Comprehensive Rust library for managing hierarchical CPS nodes on-chain
  - Beautiful CLI interface with colored output and tree visualization
  - Multi-algorithm AEAD encryption support (XChaCha20-Poly1305, AES-256-GCM, ChaCha20-Poly1305)
  - MQTT bridge for IoT device integration
  - See [libcps/README.md](tools/libcps/README.md) for detailed documentation

### Development Infrastructure

- **`nix/`** - Nix flake modules and build configurations
- **`scripts/`** - Build, deployment, and testing scripts
  - `benchmark-pallets.sh` - Automated runtime benchmarking for all pallets
  - `build-deb.sh` - Debian package builder
  - `build-runtime.sh` - Runtime WASM builder
  - `resolc` - Solang compiler wrapper for Solidity contracts
  - `docker/` - Docker configuration and healthcheck scripts
  - `weights/` - Weight template for runtime benchmarks
  - `zombienet/` - Multi-node test network configurations and integration tests

### Documentation

Full API documentation is available at https://crates.robonomics.network.

Each component is designed to be modular and reusable, following Substrate's framework architecture. The workspace structure allows for efficient development and testing of individual components while maintaining consistency across the project.

## Development

### Nix Development Shells

We provide two specialized development environments through Nix flakes:

#### Default Development Shell

For general node development, building, and testing:

```bash
# Clone the repository
git clone https://github.com/airalab/robonomics.git
cd robonomics

# Enter the development shell
nix develop
```

This shell provides:
- **Rust toolchain** - Complete Rust environment with `cargo`, `rustc`, and `rustfmt`
- **Build dependencies** - `clang`, `openssl`, `protobuf`, and other system libraries
- **Development tools**:
  - `taplo` - TOML file formatter
  - `subxt-cli` - Substrate metadata tool
  - `srtool-cli` - Deterministic WASM runtime builder
  - `psvm` - Polkadot SDK version manager
  - `frame-omni-bencher` - Benchmarking tool
  - `actionlint` - GitHub Actions workflow linter
- **Environment variables** - Pre-configured `LIBCLANG_PATH`, `PROTOC`, `RUST_SRC_PATH`

Common development tasks:

```bash
# Build in release mode
cargo build --release

# Run the node in development mode
./target/release/robonomics --dev

# Run all tests
cargo test --all

# Format code
cargo fmt

# Lint with clippy
cargo clippy --all-targets --all-features

# Format TOML files
taplo fmt
```

#### Local Testnet Shell

For multi-node testing with Zombienet:

```bash
nix develop .#local-testnet
```

This shell provides:
- **`robonomics`** - Your built Robonomics node binary
- **`polkadot`** - Polkadot relay chain binary
- **`polkadot-parachain`** - Generic parachain binary
- **`zombienet`** - Network orchestration tool for testing

Use this for testing parachain functionality with multiple collators and relay chain nodes:

```bash
# Launch a test network with zombienet
zombienet spawn scripts/zombienet/<config-file>.toml

# Run integration tests
zombienet test scripts/zombienet/<test-file>.toml
```

### Development Workflow

**Running a Local Development Node:**

The `--dev` flag starts a single-node development chain:

```bash
robonomics --dev
```

This creates:
- A local testnet with pre-funded accounts (Alice, Bob, Charlie, Dave, Eve, Ferdie)
- Temporary storage (cleared on restart)
- WebSocket RPC endpoint at `ws://127.0.0.1:9944`
- Block production every 6 seconds

**Persisting Chain Data:**

```bash
# Store chain data in a custom directory
robonomics --dev --base-path ./my-dev-chain

# Clear the chain and start fresh
robonomics --dev --base-path ./my-dev-chain purge-chain
```

**Testing Changes:**

```bash
# Run all tests
cargo test --all

# Run tests for a specific pallet
cargo test -p pallet-robonomics-datalog

# Run integration tests
cargo test --features runtime-benchmarks
```

### Runtime Benchmarking

Runtime benchmarking generates accurate weight functions for all pallets, which are crucial for accurate transaction fee calculation and preventing DoS attacks by ensuring extrinsics don't exceed block computational limits.

#### Quick Start: One-Line Benchmarking with Nix

The easiest way to run benchmarks is using the dedicated benchmarking shell:

```bash
# Enter the benchmarking shell and run all benchmarks
nix develop .#benchmarking -c ./scripts/benchmark-pallets.sh
```

This single command will:
1. Set up the complete benchmarking environment (Rust toolchain, frame-omni-bencher, etc.)
2. Build the runtime with `runtime-benchmarks` feature
3. Run benchmarks for all 18 pallets (10 system/XCM + 8 Robonomics custom)
4. Generate weight files:
   - System/XCM pallets → `runtime/robonomics/src/weights/`
   - Robonomics pallets → `frame/*/src/weights.rs`

**Customizing Benchmark Parameters:**

You can customize the benchmark steps and repeats using environment variables:

```bash
# Use fewer steps/repeats for faster testing (default: steps=50, repeat=20)
BENCHMARK_STEPS=10 BENCHMARK_REPEAT=5 nix develop .#benchmarking -c ./scripts/benchmark-pallets.sh

# Minimal settings for quick validation
BENCHMARK_STEPS=2 BENCHMARK_REPEAT=1 nix develop .#benchmarking -c ./scripts/benchmark-pallets.sh
```

#### Benchmarking Individual Pallets

To benchmark a specific pallet:

```bash
# Enter the benchmarking shell
nix develop .#benchmarking

# Benchmark a specific pallet
frame-omni-bencher v1 benchmark pallet \
  --runtime ./target/release/wbuild/robonomics-runtime/robonomics_runtime.compact.compressed.wasm \
  --pallet pallet_robonomics_datalog \
  --extrinsic "*" \
  --template ./scripts/weights/frame-weight-template.hbs \
  --output ./frame/datalog/src/weights.rs \
  --header ./LICENSE \
  --steps 50 \
  --repeat 20
```

#### Available Pallets for Benchmarking

The `benchmark-pallets.sh` script generates weights for all configured pallets:

**System Pallets** (saved to `runtime/robonomics/src/weights/`):
- `pallet_balances` - Balance transfers and reserves
- `pallet_timestamp` - Block timestamp setting
- `pallet_utility` - Batch calls and derivative dispatches
- `pallet_multisig` - Multi-signature operations
- `pallet_vesting` - Token vesting schedules
- `pallet_assets` - Asset management
- `pallet_collator_selection` - Collator selection mechanism
- `pallet_session` - Session key management

**XCM Pallets** (saved to `runtime/robonomics/src/weights/`):
- `cumulus_pallet_xcmp_queue` - Cross-chain message queue
- `pallet_xcm` - XCM message execution

**Robonomics Custom Pallets** (saved to `frame/*/src/weights.rs`):
- `pallet_robonomics_datalog` - IoT datalog storage
- `pallet_robonomics_digital_twin` - Digital twin state management
- `pallet_robonomics_launch` - Device launch commands
- `pallet_robonomics_liability` - Smart contracts for robotics
- `pallet_robonomics_rws` - RWS subscription management
- `pallet_robonomics_cps` - Cyber-physical systems integration
- `pallet_wrapped_asset` - Token wrapping functionality
- `pallet_xcm_info` - XCM integration utilities

#### Manual Benchmarking (Without Nix)

If you prefer not to use Nix:

```bash
# 1. Install frame-omni-bencher
cargo install --git https://github.com/paritytech/polkadot-sdk frame-omni-bencher

# 2. Build the runtime with benchmarking features
cargo build --release --features runtime-benchmarks -p robonomics-runtime

# 3. Run the benchmark script
./scripts/benchmark-pallets.sh
```

#### Understanding Benchmark Results

Benchmark results are written as weight functions in Rust code. For example, in `frame/datalog/src/weights.rs`:

```rust
// Example pseudocode - actual implementation uses trait methods
impl WeightInfo for SubstrateWeight<T> {
    fn record() -> Weight {
        Weight::from_parts(50_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(1))
    }
}
```

These weights are used by the runtime to:
- Calculate transaction fees accurately
- Prevent block overloading
- Ensure fair resource allocation

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](https://github.com/airalab/robonomics/blob/master/CONTRIBUTING.md).

## Support

- **Documentation**: https://wiki.robonomics.network
- **GitHub Issues**: https://github.com/airalab/robonomics/issues
- **Website**: https://robonomics.network

## License

Robonomics is licensed under the Apache License 2.0. See [LICENSE](https://github.com/airalab/robonomics/blob/master/LICENSE) for details.