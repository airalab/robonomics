# Robonomics Development Guidelines

> Crates API is available at https://crates.robonomics.network.

This repository hosts the **Robonomics Network runtime and FRAME pallets only**. The blockchain node binary and the accompanying tooling (`libcps`, `robonet`, etc.) now live in the [`airalab/robins`](https://github.com/airalab/robins) repository. Use `robins` when you need a runnable node, a local testnet, or the CLI tools; use this repository when you work on the runtime, pallets, or the type-safe `subxt-api`.

Each component is designed to be modular and reusable, following Substrate's framework architecture. The workspace structure allows for efficient development and testing of individual components while maintaining consistency across the project.

## Nix Development Shells

1. Install Nix with flakes support (one-time setup):

```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

2. We provide two specialized development environments through Nix flakes:

### Default Development Shell

For runtime and pallet development, building, and testing:

```bash
# Clone the repository
git clone https://github.com/airalab/robonomics.git
cd robonomics

# Enter the development shell
nix develop
```

This shell provides:
- **Rust toolchain** - Complete Rust environment with `cargo`, `rustc`, and `rustfmt`, pinned via `rust-toolchain.toml`
- **Build dependencies** - `clang`/`lld` (via `clangStdenv`), `openssl`, and other system libraries
- **Development tools**:
  - `taplo` - TOML file formatter
  - `actionlint` - GitHub Actions workflow linter
  - `cargo-nextest` - Next-generation test runner
  - `cargo-audit` - Security vulnerability auditing for dependencies
  - `cargo-machete` - Unused dependency detector
  - `psvm` - Polkadot SDK version manager
  - `try-runtime-cli` - Dry-run runtime upgrade tool
  - `srtool-cli` - Deterministic WASM runtime builder
  - `frame-omni-bencher` - Runtime benchmarking tool

Common development tasks:

```bash
# Build the runtime (produces the WASM artifact under target/release/wbuild/)
cargo build --release

# Run all tests
cargo nextest run

# Format code
cargo fmt

# Lint with clippy
cargo clippy --all-targets --all-features

# Format TOML files
taplo fmt
```

> Need a runnable node or a local testnet? Build and run the node from the
> [`airalab/robins`](https://github.com/airalab/robins) repository, which also
> ships the `robonet` local network orchestration tool and the CLI utilities.

### Benchmarking Shell

For generating pallet weights with `frame-omni-bencher`:

```bash
nix develop .#benchmarking
```

See [Runtime Benchmarking](#runtime-benchmarking) below for usage.

## Development Workflow

**Building the Runtime:**

This workspace builds the Robonomics runtime WASM, not a node binary:

```bash
# Build the runtime; the WASM artifact is emitted under
# target/release/wbuild/robonomics-runtime/
cargo build --release -p robonomics-runtime
```

To run the runtime inside a node — for example to start a `--dev` chain with
pre-funded accounts (Alice, Bob, Charlie, …), WebSocket RPC on
`ws://127.0.0.1:9944`, and block production — use the node binary from the
[`airalab/robins`](https://github.com/airalab/robins) repository, pointing it at
your locally built runtime WASM where needed.

**Regenerating subxt-api metadata:**

After changing the runtime, keep the type-safe `subxt-api` metadata in sync:

```bash
cargo build -p robonomics-runtime
cargo build -p robonomics-runtime-subxt-api --features build-metadata
```

See [runtime/robonomics/subxt-api/README.md](./runtime/robonomics/subxt-api/README.md) for details.

**Testing Changes:**

```bash
# Run all tests (nextest is provided by the dev shell)
cargo nextest run

# Run tests for a specific pallet
cargo test -p pallet-robonomics-datalog

# Build with runtime benchmarks enabled
cargo build --features runtime-benchmarks -p robonomics-runtime
```

## Runtime Benchmarking

Runtime benchmarking generates accurate weight functions for all pallets, which are crucial for accurate transaction fee calculation and preventing DoS attacks by ensuring extrinsics don't exceed block computational limits.

### Quick Start: One-Line Benchmarking with Nix

The easiest way to run benchmarks is using the dedicated benchmarking shell:

```bash
# Enter the benchmarking shell and run all benchmarks
nix develop .#benchmarking -c ./scripts/runtime-benchmarks.sh
```

This single command will:
1. Set up the complete benchmarking environment (Rust toolchain, frame-omni-bencher, etc.)
2. Build the runtime with `runtime-benchmarks` feature
3. Run benchmarks for all runtime pallets
4. Generate weight files → `runtime/robonomics/src/weights/`

**Customizing Benchmark Parameters:**

You can customize the benchmark steps and repeats using environment variables:

```bash
# Use fewer steps/repeats for faster testing (default: steps=50, repeat=20)
BENCHMARK_STEPS=10 BENCHMARK_REPEAT=5 nix develop .#benchmarking -c ./scripts/runtime-benchmarks.sh

# Minimal settings for quick validation
BENCHMARK_STEPS=2 BENCHMARK_REPEAT=1 nix develop .#benchmarking -c ./scripts/runtime-benchmarks.sh
```

### Benchmarking Individual Pallets

To benchmark a specific pallet:

```bash
# Enter the benchmarking shell
nix develop .#benchmarking

# Benchmark a specific pallet
frame-omni-bencher v1 benchmark pallet \
  --runtime ./target/release/wbuild/robonomics-runtime/robonomics_runtime.compact.compressed.wasm \
  --pallet pallet_robonomics_datalog \
  --extrinsic "*" \
  --output ./weights.rs \
  --header ./.github/license-check/HEADER-APACHE2 \
  --steps 50 \
  --repeat 20
```

### Manual Benchmarking (Without Nix)

If you prefer not to use Nix:

```bash
# 1. Install frame-omni-bencher
cargo install --git https://github.com/paritytech/polkadot-sdk frame-omni-bencher

# 2. Run the benchmark script
./scripts/runtime-benchmarks.sh
```

### Understanding Benchmark Results

Benchmark results are written as weight functions in Rust code. For example, in `runtime/robonomics/src/weights/pallet_robonomics_datalog.rs`:

```rust
// Example pseudocode - actual implementation uses trait methods
impl WeightInfo for WeightInfo<T> {
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

## Runtime Upgrade Testing

Before deploying runtime upgrades to production, you can dry-run them against live chain state using the `scripts/try-runtime.sh` script. This performs all migration and upgrade checks without modifying the actual chain.

### Basic Usage

Test against Live network:

```bash
./scripts/try-runtime.sh
```

### What It Does

The script:
- Connects to public RPC endpoints (wss://polkadot.rpc.robonomics.network)
- Automatically builds the runtime with `try-runtime` features if not found
- Runs `on-runtime-upgrade` checks against live chain state
- Validates that migrations execute successfully without errors
- Reports any issues before they reach production

## GitHub Actions CI/CD

The Robonomics project uses GitHub Actions for continuous integration and deployment. The CI/CD pipeline includes automated testing, building, and release workflows optimized for speed and reliability.

**Key workflows:**
- Nightly builds and artifact publishing
- Comprehensive test suite (unit tests, runtime benchmarks)
- Release pipeline with multi-platform binaries
- Docker image builds and SRTOOL runtime generation

For detailed documentation on workflow structure, caching strategies, and maintenance guidelines, see [.github/workflows/README.md](./.github/workflows/README.md).

## Development Tooling

The Robonomics node binary and its companion tools now live in the
[`airalab/robins`](https://github.com/airalab/robins) repository. They are no
longer part of this runtime-only workspace.

### libcps - CPS Library & CLI

A comprehensive library and command-line interface for managing hierarchical Cyber-Physical Systems on Robonomics. Features multi-algorithm encryption, dual keypair support, and MQTT bridge for IoT integration.

Now maintained in [`airalab/robins`](https://github.com/airalab/robins).

### robonet - Network Testbed

Built on ZombieNet SDK, `robonet` provides an easy way to spawn local Robonomics networks for integration testing. Supports multiple network topologies and includes comprehensive tests for XCM, CPS, and other pallets.

Now maintained in [`airalab/robins`](https://github.com/airalab/robins).

### subxt-api - Type-safe Runtime API

A type-safe, compile-time verified API for the Robonomics runtime, generated from runtime metadata using [subxt](https://docs.rs/subxt). This crate stays in this repository alongside the runtime it mirrors.

Documentation: [runtime/robonomics/subxt-api/README.md](./runtime/robonomics/subxt-api/README.md)

## Nix Workflow and Binary Cache

This project uses [Nix](https://nixos.org/) to provide reproducible development environments with all necessary dependencies pre-configured. The Nix flake defines multiple development shells optimized for different workflows (see [Nix Development Shells](#nix-development-shells) above).

### Using the Robonomics Binary Cache

To speed up builds, the project uses [Cachix](https://cachix.org/) to cache pre-built Nix artifacts. This eliminates the need to build dependencies from source:

**Setup Cachix cache (one-time):**

```bash
# Install cachix
nix-env -iA cachix -f https://cachix.org/api/v1/install

# Add the Robonomics cache
cachix use robonomics
```

After setup, Nix will automatically download pre-built binaries from `robonomics.cachix.org` instead of building from source, significantly reducing build times.

**Note:** The CI/CD pipeline automatically builds and uploads artifacts to Cachix on every master branch push, ensuring the cache stays up-to-date with the latest changes.
