[<img align="right" src="https://github.com/airalab/robonomics/blob/master/web3_foundation_grants_badge_black.jpg">](https://medium.com/web3foundation/web3-foundation-grants-wave-two-recipients-16d9b996501d)

# Robonomics Network Runtime

[![License](https://img.shields.io/github/license/airalab/robonomics)](https://github.com/airalab/robonomics/blob/master/LICENSE)
[![Release](https://img.shields.io/github/release/airalab/robonomics.svg)](https://github.com/airalab/robonomics/releases)
[![Nightly](https://github.com/airalab/robonomics/workflows/Nightly/badge.svg)](https://github.com/airalab/robonomics/actions/workflows/nightly.yml)
[![Downloads](https://img.shields.io/github/downloads/airalab/robonomics/total.svg)](https://github.com/airalab/robonomics/releases)
[![Matrix](https://img.shields.io/matrix/robonomics:matrix.org)](https://matrix.to/#/#robonomics:matrix.org)

> Robonomics in based on the [Polkadot SDK](https://polkadot.com/platform/sdk/). For more specific guides, like how to be a node, see the [Robonomics Wiki](https://wiki.robonomics.network).

Robonomics is a set of open-source packages and infrastructure for Robotics, Smart Cities and Industry 4.0.

## Quick Start - Choose Your Destiny...

| Code Builder | Network Guard |
| --- | --- |
|[<img src="https://github.com/user-attachments/assets/fc522054-48de-4f6d-a913-f9204f8047bc">](./DEVELOPER_GUIDE.md)|[<img src="https://github.com/user-attachments/assets/ee3b6d88-da2c-4620-b72f-92ca7fdef7a6">](./COLLATOR_GUIDE.md)|

## Repository Structure

This repository is organized as a Cargo workspace with the following structure:

### Runtime

- **`runtime/`**
  - `robonomics/` - WASM runtime for the Robonomics Network
  - `robonomics/subxt-api` - a type-safe, compile-time verified API based on [subxt](https://docs.rs/subxt/latest/subxt/) library interface.

### Pallets

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

### Development Infrastructure

- **`scripts/`** - Build, deployment, and testing scripts
  - `weights/` - Weight template for runtime benchmarks
  - `runtime-benchmarks.sh` - Automated runtime benchmarking for all pallets
  - `try-runtime.sh` - Automated runtime upgrade checks

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](https://github.com/airalab/robonomics/blob/master/CONTRIBUTING.md).

## Support

- **Robonomics Wiki**: https://wiki.robonomics.network
- **GitHub Issues**: https://github.com/airalab/robonomics/issues
- **Website**: https://robonomics.network

## License

Robonomics is licensed under the Apache License 2.0. See [LICENSE](./LICENSE) for details.
