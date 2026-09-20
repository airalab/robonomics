[<img align="right" src="https://github.com/airalab/robonomics/blob/master/web3_foundation_grants_badge_black.jpg">](https://medium.com/web3foundation/web3-foundation-grants-wave-two-recipients-16d9b996501d)

# Robonomics

[![License](https://img.shields.io/github/license/airalab/robonomics)](https://github.com/airalab/robonomics/blob/master/LICENSE)
[![Release](https://img.shields.io/github/release/airalab/robonomics.svg)](https://github.com/airalab/robonomics/releases)
[![Nightly](https://github.com/airalab/robonomics/workflows/Nightly/badge.svg)](https://github.com/airalab/robonomics/actions/workflows/nightly.yml)

> Robonomics is based on the [Polkadot SDK](https://polkadot.com/platform/sdk/). For more specific guides, like how to be a node, see the [Robonomics Wiki](https://wiki.robonomics.network).

Robonomics is a set of open-source packages and infrastructure for Robotics, Smart Cities and Industry 4.0.

## Quick Start - Choose Your Destiny...

| Code Builder | Network Guard |
| --- | --- |
|[<img src="https://github.com/user-attachments/assets/fc522054-48de-4f6d-a913-f9204f8047bc">](./DEVELOPER_GUIDE.md)|[<img src="https://github.com/user-attachments/assets/ee3b6d88-da2c-4620-b72f-92ca7fdef7a6">](./COLLATOR_GUIDE.md)|

## Repository Structure

Since release 50 this is the **runtime-only** repository: runtime, pallets, protocol logic,
chain specs and runtime upgrades. Node binaries and operational tooling live in
[`airalab/robins`](https://github.com/airalab/robins).

```text
airalab/robonomics        airalab/robins
  runtime                   blockchain node
  pallets                   utilities
  chain specs               service tooling
```

Collators run the generic [`polkadot-omni-node`](https://crates.io/crates/polkadot-omni-node)
driven by a chain spec — see [COLLATOR_GUIDE.md](./COLLATOR_GUIDE.md).

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

- **`chain-spec/`** - the `robonomics-chain-spec` crate
  - Embeds the raw Kusama and Polkadot parachain chain specs as `&'static str` constants
  - Has no dependencies of its own, so downstream tooling and collator setups can depend on
    it instead of vendoring JSON or fetching it from GitHub

### Documentation

- **`docs/robonomics-5.0-roadmap.md`** - the Robonomics 5.0 architecture and implementation
  sequence: Governance, CPS, Scope, Access, Subscription, Storage, Compute and Policy
- **[`COLLATOR_GUIDE.md`](./COLLATOR_GUIDE.md)** - running a collator on `polkadot-omni-node`
- **[`DEVELOPER_GUIDE.md`](./DEVELOPER_GUIDE.md)** - building and working with the runtime

### Development Infrastructure

- **`scripts/`** - Build, deployment, and testing scripts
  - `weights/` - Weight template for runtime benchmarks
  - `build-runtime.sh` - Deterministic runtime build via `paritytech/srtool`
  - `runtime-benchmarks.sh` - Automated runtime benchmarking for all pallets
  - `check-weights.pl` - Fails if an extrinsic is not charged through a benchmarked `WeightInfo`
  - `try-runtime.sh` - Automated runtime upgrade checks

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](https://github.com/airalab/robonomics/blob/master/CONTRIBUTING.md).

## Support

- **Robonomics Wiki**: https://wiki.robonomics.network
- **GitHub Issues**: https://github.com/airalab/robonomics/issues
- **Website**: https://robonomics.network

## License

Robonomics is licensed under the Apache License 2.0. See [LICENSE](./LICENSE) for details.
