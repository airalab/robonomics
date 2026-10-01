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

> Need a local testnet?
> - see [Running a Development Chain](#running-a-development-chain) below.

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
`ws://127.0.0.1:9944`, and block production — you can either use the generic
`polkadot-omni-node` binary as described in
[Running a Development Chain](#running-a-development-chain) below, or the
node binary from the [`airalab/robins`](https://github.com/airalab/robins)
repository, pointing it at your locally built runtime WASM where needed.

**Regenerating runtime metadata:**

After changing the runtime, keep the committed runtime metadata (used by
`subxt-api` and future API generators) in sync:

```bash
cargo build -p robonomics-runtime
cargo build -p robonomics-runtime-metadata --features build-metadata
```

See [runtime/robonomics/metadata/README.md](./runtime/robonomics/metadata/README.md) for details.

**Testing Changes:**

```bash
# Run all tests (nextest is provided by the dev shell)
cargo nextest run

# Run tests for a specific pallet
cargo test -p pallet-robonomics-datalog

# Build with runtime benchmarks enabled
cargo build --features runtime-benchmarks -p robonomics-runtime
```

## Running a Development Chain

The `nix develop` shell ships with [`chain-spec-builder`](https://crates.io/crates/staging-chain-spec-builder) and
[`polkadot-omni-node`](https://crates.io/crates/polkadot-omni-node), which together are enough to build a chain spec
from your locally compiled runtime and run it as a single-node development chain with block production,
pre-funded dev accounts, and RPC — all without a relay chain or other collators.

The [`scripts/development-chain.sh`](./scripts/development-chain.sh) helper wires these tools together.
On every invocation it:

1. Checks that the required binaries are installed. If not, it prints how to install them
   (or how to use Nix flakes instead) and exits.
2. Uses the runtime WASM from `RUNTIME_WASM` if set; otherwise uses
   `target/release/wbuild/robonomics-runtime/robonomics_runtime.compact.compressed.wasm`,
   running `cargo build --release -p robonomics-runtime` first if it is missing.
3. Calls `chain-spec-builder` to generate a temporary `dev.json` chain spec (chain id `robonomics-dev`,
   para id `2000`, relay chain `westend-local`, token `XRT` with 9 decimals) from the runtime's
   `development` genesis preset.
4. Runs the action passed as the first argument (`run` or `docker`, see below).

The spec lives in a temporary directory that is removed when the script exits, so nothing is written
to your working tree.

> Running the script without an argument only generates the spec; it does not start a node
> or build an image. Pass `run` or `docker` to do something useful with it.

### Run the Chain in One Line

```bash
nix develop -c ./scripts/development-chain.sh run
```

This builds the runtime if needed, generates the spec, and starts
`polkadot-omni-node --chain <spec> --dev`, which gives you:
- Block production out of the box (no need to insert session keys manually)
- JSON-RPC / WebSocket endpoints on `ws://127.0.0.1:9944`
- Pre-funded dev accounts (Alice, Bob, Charlie, …)
- An ephemeral database that is wiped when the process exits

Use this as your fast local feedback loop while iterating on pallets and runtime logic.
Stop the node with `Ctrl+C`.

### Build a Local Docker Image in One Line

```bash
nix develop -c ./scripts/development-chain.sh docker
```

This generates the same spec and builds an image from [`docker/Dockerfile`](./docker/Dockerfile), which is based on
`parity/polkadot-omni-node` and bakes in the generated `dev.json` (plus `chain-spec/polkadot-parachain.raw.json`
as `parachain.json`). The image's default command is `--chain=/dev.json --dev`.
Docker must be installed and its daemon running (Nix does not provide it).

The script runs a plain `docker build`, so the resulting image is untagged. Tag the most recently
created image and run it:

```bash
docker tag "$(docker images -q | head -n 1)" robonomics-dev

# Default command: --chain=/dev.json --dev
docker run --rm -p 9944:9944 robonomics-dev
```

Arguments after the image name replace the default command entirely, so repeat the defaults when adding
flags, e.g. to accept RPC connections from outside the container:

```bash
docker run --rm -p 9944:9944 robonomics-dev --chain=/dev.json --dev --rpc-external
```

### Without Nix

Every script checks for the binaries it needs and tells you what is missing. Without Nix, install them
with Cargo (`cargo install staging-chain-spec-builder polkadot-omni-node`) and run the script directly:

```bash
./scripts/development-chain.sh run
```

### Using a Pre-built Runtime

If you already have a runtime WASM built elsewhere (e.g. a different profile or a `srtool` build),
point the script at it instead of rebuilding. This works for both actions:

```bash
RUNTIME_WASM=/path/to/robonomics_runtime.compact.compressed.wasm \
  nix develop -c ./scripts/development-chain.sh run
```

### Running the Node Manually

To keep the chain spec around or customize node flags, call the tools directly:

```bash
chain-spec-builder -c ./chain_spec.json create \
  -n "Robonomics Dev" -i robonomics-dev -t development \
  -r ./target/release/wbuild/robonomics-runtime/robonomics_runtime.compact.compressed.wasm \
  --para-id 2000 --relay-chain westend-local \
  --properties tokenSymbol=XRT,tokenDecimals=9 \
  named-preset development

polkadot-omni-node --chain ./chain_spec.json --dev
```

`polkadot-omni-node` is runtime-agnostic — it starts a node purely from a chain spec, so no
Robonomics-specific binary is required. Pass `--base-path <dir>` if you want state to persist across restarts.

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
