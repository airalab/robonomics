[<img align="right" src="https://github.com/airalab/robonomics/blob/master/web3_foundation_grants_badge_black.jpg">](https://medium.com/web3foundation/web3-foundation-grants-wave-two-recipients-16d9b996501d)

# Robonomics Network

[![License](https://img.shields.io/github/license/airalab/robonomics)](https://github.com/airalab/robonomics/blob/master/LICENSE)
[![Release](https://img.shields.io/github/release/airalab/robonomics.svg)](https://github.com/airalab/robonomics/releases)
[![Nightly](https://github.com/airalab/robonomics/workflows/Nightly/badge.svg)](https://github.com/airalab/robonomics/actions/workflows/nightly.yml)
[![Downloads](https://img.shields.io/github/downloads/airalab/robonomics/total.svg)](https://github.com/airalab/robonomics/releases)
[![Matrix](https://img.shields.io/matrix/robonomics:matrix.org)](https://matrix.to/#/#robonomics:matrix.org)

> Robonomics implementation in Rust based on the [Polkadot SDK](https://polkadot.com/platform/sdk/). For more specific guides, like how to be a node, see the [Robonomics Wiki](https://wiki.robonomics.network).

Robonomics platform includes a set of open-source packages and infrastructure for Robotics, Smart Cities and Industry 4.0 developers.

## Quick Start - Choose Your Destiny...

| Code Builder | Network Guard |
| --- | --- |
|[<img src="https://github.com/user-attachments/assets/fc522054-48de-4f6d-a913-f9204f8047bc" width="30%">](./DEVELOPER_GUIDE.md)|[<img src="https://github.com/user-attachments/assets/02b4152c-7f14-4896-a406-55e60f6362e6" width="30%">](./COLLATOR_GUIDE.md)|

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

- **`runtime/robonomics/subxt-api`** - Robonomics runtime Subxt metadata & API
  - Provides a type-safe, compile-time verified API based on [subxt](https://docs.rs/subxt/latest/subxt/) library interface.

### Custom Pallets

- **`frame/`** - Custom FRAME pallets for IoT and robotics
  - `datalog/` - Immutable on-chain data logging with time-series storage
  - `digital-twin/` - Digital twin state management and topic-based data organization
  - `launch/` - Robot/device launch commands with parameter support
  - `liability/` - Smart contract-like agreements for robotics tasks
  - `rws/` - Robonomics Web Services (RWS) subscription management
  - `cps/` - Cyber-physical Systems pallet for IoT integration
  - `parachain-info/` - Original cumulus pallet extended with relay network info
  - `collator-rewards/` - Generic per-block collator (block author) reward helper

### Chain Specifications

- **`chains/`** - Chain specification files for different networks

### Tools

- **`tools/libcps/`** - Robonomics CPS (Cyber-Physical Systems) library and CLI
  - Comprehensive Rust library for managing hierarchical CPS nodes on-chain
  - Beautiful CLI interface with colored output and tree visualization
  - Multi-algorithm AEAD encryption support (XChaCha20-Poly1305, AES-256-GCM, ChaCha20-Poly1305)
  - MQTT bridge for IoT device integration
  - See [libcps/README.md](tools/libcps/README.md) for detailed documentation

- **`tools/robonet/`** - Local network spawner and integration test framework
  - CLI tool for spawning multi-node test networks using ZombieNet SDK
  - Built-in integration tests for XCM, CPS, and network functionality
  - Multiple network topologies (simple parachain, with AssetHub for XCM testing)
  - Developer-friendly interface with progress indicators and detailed logging
  - See [robonet/README.md](tools/robonet/README.md) for detailed documentation

### Development Infrastructure

- **`nix/`** - Nix flake modules and build configurations
- **`scripts/`** - Build, deployment, and testing scripts
  - `runtime-benchmarks.sh` - Automated runtime benchmarking for all pallets
  - `try-runtime.sh` - Automated runtime upgrade checks
  - `build-deb.sh` - Debian package builder
  - `build-runtime.sh` - Deterministic runtime WASM builder
  - `docker/` - Docker configuration and healthcheck scripts
  - `weights/` - Weight template for runtime benchmarks

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](https://github.com/airalab/robonomics/blob/master/CONTRIBUTING.md).

## Support

- **Robonomics Wiki**: https://wiki.robonomics.network
- **GitHub Issues**: https://github.com/airalab/robonomics/issues
- **Website**: https://robonomics.network

## License

Robonomics is licensed under the Apache License 2.0. See [LICENSE](./LICENSE) for details.
