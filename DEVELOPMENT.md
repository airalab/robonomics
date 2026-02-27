# Robonomics Development Guidelines

## Nix Development Shells

We provide two specialized development environments through Nix flakes:

### Default Development Shell

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
  - `try-runtime` - Dry-run runtime upgrade tool
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

### Local Testnet Shell

For multi-node testing with Zombienet:

```bash
nix develop .#robonet
```

This shell provides:
- **`robonomics`** - Your built Robonomics node binary
- **`polkadot`** - Polkadot relay chain binary
- **`polkadot-parachain`** - Generic parachain binary
- **`robonet`** - ZombienetSDK based local networks orchestration tool

Use this for testing parachain functionality with multiple collators and relay chain nodes:

```bash
# Launch a test network
robonet spawn

# Run integration tests
robonet test
```

Detailed **robonet** documentation available at crate [README](./tools/robonet/README.md).

## Development Workflow

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
