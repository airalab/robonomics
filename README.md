# Robonomics Network

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
  - `cps/` - Cyber-physical Systems pallet for IoT integration
  - `claim/` - Pallet for ERC20 token claim support
  - `parachain-info/` - Original cumulus pallet extended with relay network info

### Chain Specifications

- **`chains/`** - Chain specification files for different networks

### Tools

- **`tools/robonet/`** - Local network spawner and integration test framework
  - CLI tool for spawning multi-node test networks using ZombieNet SDK
  - Built-in integration tests for XCM, CPS, Claim pallets, and network functionality
  - Multiple network topologies (simple parachain, with AssetHub for XCM testing)
  - Developer-friendly interface with progress indicators and detailed logging
  - See [robonet/README.md](tools/robonet/README.md) for detailed documentation

- **`tools/libcps/`** - Robonomics CPS (Cyber-Physical Systems) library and CLI
  - Comprehensive Rust library for managing hierarchical CPS nodes on-chain
  - Beautiful CLI interface with colored output and tree visualization
  - Multi-algorithm AEAD encryption support (XChaCha20-Poly1305, AES-256-GCM, ChaCha20-Poly1305)
  - MQTT bridge for IoT device integration
  - See [libcps/README.md](tools/libcps/README.md) for detailed documentation

### Development Infrastructure

- **`nix/`** - Nix flake modules and build configurations
- **`scripts/`** - Build, deployment, and testing scripts
  - `runtime-benchmarks.sh` - Automated runtime benchmarking for all pallets
  - `try-runtime.sh` - Automated runtime upgrade checks
  - `build-deb.sh` - Debian package builder
  - `build-runtime.sh` - Deterministic runtime WASM builder
  - `docker/` - Docker configuration and healthcheck scripts
  - `weights/` - Weight template for runtime benchmarks

### Documentation

Development guidelines available at [DEVELOPMENT.md](./DEVELOPMENT.md).

Crates API is available at https://crates.robonomics.network.

Each component is designed to be modular and reusable, following Substrate's framework architecture. The workspace structure allows for efficient development and testing of individual components while maintaining consistency across the project.

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](https://github.com/airalab/robonomics/blob/master/CONTRIBUTING.md).

## Support

- **Robonomics Wiki**: https://wiki.robonomics.network
- **GitHub Issues**: https://github.com/airalab/robonomics/issues
- **Website**: https://robonomics.network

## License

Robonomics is licensed under the Apache License 2.0. See [LICENSE](./LICENSE) for details.
