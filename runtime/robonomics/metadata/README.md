# Robonomics Runtime Metadata

The single source of truth for SCALE-encoded Robonomics runtime metadata.

This crate is responsible only for obtaining, validating, and exposing
runtime metadata as a reusable build artifact. It does not know or care how
that metadata is used - consumers (such as the Rust [`subxt-api`](../subxt-api)
crate, or a future C++/embedded code generator for `esp-robonomics-client`)
depend on this crate and read [`METADATA`](./src/lib.rs) without needing to
know how it was produced.

## Overview

This crate provides:
- **Automatic metadata extraction** from the Robonomics runtime during compilation
- **A single build artifact** (`METADATA: &[u8]`) reusable by any consumer
- **Consistency checking** against the committed metadata via `check-metadata`
- **Minimal dependencies** by default (no runtime dependencies at all)

## How It Works

The crate supports three build modes.

### 1. Using Prebuilt Metadata (Default - Faster)

By default, the build uses the prebuilt [`metadata.scale`](./metadata.scale) file committed to the repository. This is the **fastest** option and doesn't require building the runtime:

```
┌─────────────────────────────────────────────────────────────┐
│              Fast Build (Default Mode)                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. build.rs copies metadata.scale from repository          │
│     ↓                                                       │
│  2. Copies to $OUT_DIR/metadata.scale                       │
│     ↓                                                       │
│  3. `METADATA` embeds the file contents via include_bytes!  │
│     ↓                                                       │
│  ✓  Metadata ready for consumers                            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 2. Building Metadata from Runtime (Feature: `build-metadata`)

When the `build-metadata` feature is enabled, metadata is extracted directly from the runtime WASM:

```
┌─────────────────────────────────────────────────────────────┐
│           Build from Runtime (build-metadata)               │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. build.rs loads runtime WASM from robonomics-runtime     │
│     ↓                                                       │
│  2. Creates RuntimeBlob and WasmExecutor                    │
│     ↓                                                       │
│  3. Executes Metadata_metadata host function                │
│     ↓                                                       │
│  4. Decodes and validates SCALE-encoded metadata            │
│     ↓                                                       │
│  5. Saves metadata.scale to $OUT_DIR/                       │
│     ↓                                                       │
│  6. `METADATA` embeds the file contents via include_bytes!  │
│     ↓                                                       │
│  ✓  Metadata ready for consumers                            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 3. Checking Metadata Integrity (Feature: `check-metadata`)

The `check-metadata` feature verifies that the prebuilt metadata matches the runtime:

```
┌─────────────────────────────────────────────────────────────┐
│              Metadata Check (check-metadata)                │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. Requires build-metadata feature to be enabled           │
│     ↓                                                       │
│  2. Extracts metadata from runtime WASM                     │
│     ↓                                                       │
│  3. Computes SeaHash (u64) of extracted metadata            │
│     ↓                                                       │
│  4. Computes SeaHash (u64) of prebuilt metadata.scale       │
│     ↓                                                       │
│  5. Compares digests                                        │
│     ↓                                                       │
│  ✗  Panics if mismatch detected                             │
│  ✓  Continues if metadata is in sync                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Public API

```rust
/// SCALE-encoded Robonomics runtime metadata used by consumers.
pub static METADATA: &[u8] = /* ... */;
```

```rust
let metadata: &[u8] = robonomics_runtime_metadata::METADATA;
```

Consumers should not need access to this crate's `OUT_DIR` - `METADATA`
always represents the metadata selected by the crate's active feature
configuration.

## Build Features

### Default Build (No Features)

The fastest option - uses the prebuilt `metadata.scale` file:

```bash
cargo build -p robonomics-runtime-metadata
```

**When to use:**
- Normal development and testing
- CI/CD pipelines where speed matters
- When runtime hasn't changed

### `build-metadata` Feature

Extracts fresh metadata from the runtime WASM:

```bash
cargo build -p robonomics-runtime-metadata --features build-metadata
```

**When to use:**
- After modifying runtime code
- When you need to update the prebuilt metadata.scale
- To ensure metadata is in sync with runtime

**Note:** This requires the runtime to build first:
```bash
cargo build -p robonomics-runtime
cargo build -p robonomics-runtime-metadata --features build-metadata
```

### `check-metadata` Feature

Validates that prebuilt metadata matches the current runtime:

```bash
cargo build -p robonomics-runtime-metadata --features check-metadata
```

**When to use:**
- In CI/CD to ensure metadata is up to date
- Before releases to validate integrity
- After runtime changes to verify updates

**Behavior:**
- Extracts metadata from runtime WASM
- Computes SeaHash (u64) of extracted metadata and of the prebuilt `metadata.scale`
- **Panics with mismatch error** if hashes don't match
- Succeeds silently if hashes match

## Updating Prebuilt Metadata

When you modify the runtime, update the prebuilt metadata:

```bash
# 1. Build runtime first
cargo build -p robonomics-runtime

# 2. Extract metadata
cargo build -p robonomics-runtime-metadata --features build-metadata

# 3. Copy metadata to repository
cp target/debug/build/robonomics-runtime-metadata-*/out/metadata.scale \
   runtime/robonomics/metadata/metadata.scale

# 4. Verify it works
cargo build -p robonomics-runtime-metadata --features check-metadata

# 5. Commit the updated metadata
git add runtime/robonomics/metadata/metadata.scale
git commit -m "chore: update runtime metadata"
```

## Usage

### As a Dependency

Add to your `Cargo.toml`:

```toml
[build-dependencies]
robonomics-runtime-metadata = { path = "runtime/robonomics/metadata" }
# or from workspace
robonomics-runtime-metadata.workspace = true
```

### Example

```rust
let metadata = subxt_metadata::Metadata::decode_from(
    robonomics_runtime_metadata::METADATA
)?;
```

## Feature Forwarding

Downstream crates (such as `subxt-api`) may preserve their own public feature
interface by forwarding features to this crate instead of reimplementing the
metadata build logic:

```toml
[features]
build-metadata = ["robonomics-runtime-metadata/build-metadata"]
check-metadata = ["robonomics-runtime-metadata/check-metadata"]
```

## Troubleshooting

**Error**: `Metadata hash mismatch`

**Solution**: The prebuilt metadata is out of sync with the runtime. Update it (see [Updating Prebuilt Metadata](#updating-prebuilt-metadata) above).

---

**Error**: `WASM_BINARY is not available`

**Solution**: Ensure `robonomics-runtime` builds successfully first:
```bash
cargo build -p robonomics-runtime
cargo build -p robonomics-runtime-metadata --features build-metadata
```

---

**Error**: `Unable to create RuntimeBlob from WASM`

**Solution**: The runtime WASM may be corrupted. Clean and rebuild:
```bash
cargo clean -p robonomics-runtime
cargo build -p robonomics-runtime
cargo build -p robonomics-runtime-metadata --features build-metadata
```

---

**Error**: `Invalid metadata magic sequence`

**Solution**: The metadata format may have changed. This is usually a bug - report it.

## License

Apache-2.0 - See [LICENSE](../../../LICENSE) for details.
