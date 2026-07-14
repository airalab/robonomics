# Robonomics Runtime Subxt API

A type-safe API generator for the Robonomics runtime that extracts metadata at build time and generates compile-time verified blockchain interactions using [subxt](https://github.com/paritytech/subxt).

## Overview

This crate provides:
- **Automatic metadata extraction** from the Robonomics runtime during compilation
- **Type-safe API generation** using subxt's macro system
- **Minimal dependencies** compared to embedding runtime WASM directly
- **Always synchronized** metadata that matches your runtime version
- **Custom configuration** (`RobonomicsConfig`) tailored for Robonomics nodes

## How It Works

The crate supports two build modes:

### 1. Using Prebuilt Metadata (Default - Faster)

By default, the build uses a prebuilt `metadata.scale` file committed to the repository. This is the **fastest** option and doesn't require building the runtime:

```
┌─────────────────────────────────────────────────────────────┐
│              Fast Build (Default Mode)                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. build.rs copies metadata.scale from repository          │
│     ↓                                                       │
│  2. Copies to $OUT_DIR/metadata.scale                       │
│     ↓                                                       │
│  3. subxt macro reads metadata and generates types          │
│     ↓                                                       │
│  ✓  Type-safe API ready to use                              │
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
│  6. subxt macro reads metadata and generates types          │
│     ↓                                                       │
│  ✓  Type-safe API ready to use                              │
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

### Why This Approach?

**Traditional Method:**
- Embed runtime WASM in subxt macro: `#[subxt::subxt(runtime_metadata_insecure_url = "...")]`
- Pull in all runtime dependencies (hundreds of crates)
- Slow compilation, large binary size

**Our Method:**
- Use prebuilt metadata by default: **instant builds**, no runtime dependencies
- Extract metadata with `build-metadata` feature when needed
- Save to a file: `$OUT_DIR/metadata.scale`
- Reference in subxt macro: `runtime_metadata_path = "$OUT_DIR/metadata.scale"`
- **Result**: Fewer dependencies, faster builds, smaller binaries

## Build Features

### Default Build (No Features)

The fastest option - uses the prebuilt `metadata.scale` file:

```bash
# Fast build using prebuilt metadata
cargo build -p robonomics-runtime-subxt-api
```

**When to use:**
- Normal development and testing
- CI/CD pipelines where speed matters
- When runtime hasn't changed

### `build-metadata` Feature

Extracts fresh metadata from the runtime WASM:

```bash
# Build with metadata extraction
cargo build -p robonomics-runtime-subxt-api --features build-metadata
```

**When to use:**
- After modifying runtime code
- When you need to update the prebuilt metadata.scale
- To ensure metadata is in sync with runtime

**Note:** This requires the runtime to build first:
```bash
cargo build -p robonomics-runtime
cargo build -p robonomics-runtime-subxt-api --features build-metadata
```

### `check-metadata` Feature

Validates that prebuilt metadata matches the current runtime:

```bash
# Check metadata integrity (implies build-metadata)
cargo build -p robonomics-runtime-subxt-api --features check-metadata
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
cargo build -p robonomics-runtime-subxt-api --features build-metadata

# 3. Copy metadata to repository
cp target/debug/build/robonomics-runtime-subxt-api-*/out/metadata.scale \
   runtime/robonomics/subxt-api/metadata.scale

# 4. Verify it works
cargo build -p robonomics-runtime-subxt-api --features check-metadata

# 5. Commit the updated metadata
git add runtime/robonomics/subxt-api/metadata.scale
git commit -m "chore: update subxt-api metadata"
```

## Usage

### As a Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
robonomics-runtime-subxt-api = { path = "runtime/robonomics/subxt-api" }
# or from workspace
robonomics-runtime-subxt-api.workspace = true
```

### Basic Example

```rust
use robonomics_runtime_subxt_api::{api, RobonomicsConfig, AccountId32};
use subxt::OnlineClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Robonomics node
    let client = OnlineClient::<RobonomicsConfig>::from_url("ws://127.0.0.1:9988").await?;
    
    // Query chain state - e.g., get system account info
    let alice: AccountId32 = subxt_signer::sr25519::dev::alice().public_key().into();
    let at_block = client.at_current_block().await?;
    let account_info = at_block
        .storage()
        .try_fetch(api::storage().system().account(), (alice,))
        .await?;
    
    println!("Alice's account info: {:?}", account_info);
    
    Ok(())
}
```

### Submitting Transactions

```rust
use robonomics_runtime_subxt_api::{api, RobonomicsConfig};
use subxt::OnlineClient;
use subxt_signer::sr25519::dev;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OnlineClient::<RobonomicsConfig>::from_url("ws://127.0.0.1:9988").await?;
    let alice = dev::alice();
    
    // Create a remark transaction
    let remark = vec![1, 2, 3, 4];
    let tx = api::tx().system().remark(remark);
    
    // Sign and submit
    let hash = client
        .tx()
        .await?
        .sign_and_submit_default(&tx, &alice)
        .await?;
    
    println!("Transaction submitted with hash: {:?}", hash);
    
    Ok(())
}
```

### Working with CPS Pallet

```rust
use robonomics_runtime_subxt_api::{api, RobonomicsConfig};
use subxt::OnlineClient;
use subxt_signer::sr25519::dev;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OnlineClient::<RobonomicsConfig>::from_url("ws://127.0.0.1:9988").await?;
    let alice = dev::alice();
    
    // Create a CPS node with plain data
    let node_data = "Hello from CPS!";
    let create_tx = api::tx().cps().create(None, node_data.into());
    
    let result = client
        .tx()
        .await?
        .sign_and_submit_then_watch_default(&create_tx, &alice)
        .await?
        .wait_for_finalized_success()
        .await?;
    
    println!("Node created in block: {:?}", result.block_hash());
    
    // Query the node
    let node_id = 0; // Your node ID
    let at_block = client.at_current_block().await?;
    let node = at_block
        .storage()
        .try_fetch(api::storage().cps().nodes(), (node_id,))
        .await?;
    
    println!("Node data: {:?}", node);
    
    Ok(())
}
```

### Monitoring Events

```rust
use robonomics_runtime_subxt_api::{api, RobonomicsConfig};
use subxt::OnlineClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OnlineClient::<RobonomicsConfig>::from_url("ws://127.0.0.1:9988").await?;
    
    // Subscribe to finalized blocks
    let mut blocks = client.stream_blocks().await?;
    
    while let Some(block) = blocks.next().await {
        let block = block?;
        println!("Block #{}", block.number());
        
        // Process events in this block
        let at_block = block.at().await?;
        let events = at_block.events().fetch().await?;
        for event in events.iter() {
            let event = event?;
            println!("  Event: {}::{}", 
                event.pallet_name(), 
                event.event_name()
            );
            
            // Handle specific events
            if let Some(Ok(transfer)) = events.find::<api::balances::events::Transfer>().next() {
                println!("    Transfer: {:?} -> {:?}, amount: {:?}",
                    transfer.from, transfer.to, transfer.amount);
            }
        }
    }
    
    Ok(())
}
```

## API Structure

The generated API follows this structure:

```rust
use robonomics_runtime_subxt_api::api;

// Storage queries (keys are supplied to `fetch`/`try_fetch`, not the address)
api::storage().system().account();
api::storage().cps().nodes();
api::storage().claim().claims();

// Transactions
api::tx().system().remark(data);
api::tx().cps().create(parent, data);
api::tx().balances().transfer_allow_death(dest, value);

// Constants
api::constants().system().block_length();
api::constants().timestamp().minimum_period();

// Events
api::balances::events::Transfer { from, to, amount };
api::cps::events::NodeCreated { node_id, owner };
```

## Configuration

### RobonomicsConfig

The `RobonomicsConfig` type is pre-configured for Robonomics nodes. Since
subxt 0.50 the [`subxt::Config`] trait is implemented on a *value* (the client
instantiates it, hence the `Default` bound and the newtype wrapper around
`SubstrateConfig` that caches chain state such as genesis hash and metadata):

```rust
#[derive(Clone, Debug, Default)]
pub struct RobonomicsConfig(SubstrateConfig);

impl subxt::Config for RobonomicsConfig {
    type AccountId = AccountId32;           // Standard SS58 accounts
    type Signature = MultiSignature;        // Supports multiple signature types
    type Hasher = <SubstrateConfig as subxt::Config>::Hasher; // Delegated to SubstrateConfig
    type Header = SubstrateHeader<u32>;     // Standard Substrate header
    type AssetId = u32;                     // Asset ID type
    type Address = MultiAddress<AccountId32, ()>;  // Address format
    type TransactionExtensions = RobonomicsTransactionExtensions<Self>;

    // Stateful methods (genesis_hash, metadata_for_spec_version, ...) are
    // delegated to the inner SubstrateConfig.
}
```

### Custom Derives

The crate includes custom derives for CPS pallet types:

```rust
// NodeData helper implementations
impl From<Vec<u8>> for NodeData { /* ... */ }
impl From<String> for NodeData { /* ... */ }
impl From<&str> for NodeData { /* ... */ }

// Create AEAD encrypted data
NodeData::aead_from(encrypted_bytes);
```

## Troubleshooting

### Build Errors

**Error**: `Metadata SHA256 mismatch`

**Solution**: The prebuilt metadata is out of sync with the runtime. Update it:
```bash
cargo build -p robonomics-runtime
cargo build -p robonomics-runtime-subxt-api --features build-metadata
cp target/debug/build/robonomics-runtime-subxt-api-*/out/metadata.scale \
   runtime/robonomics/subxt-api/metadata.scale
```

---

**Error**: `WASM_BINARY is not available`

**Solution**: Ensure `robonomics-runtime` builds successfully first:
```bash
cargo build -p robonomics-runtime
cargo build -p robonomics-runtime-subxt-api --features build-metadata
```

---

**Error**: `Unable to create RuntimeBlob from WASM`

**Solution**: The runtime WASM may be corrupted. Clean and rebuild:
```bash
cargo clean -p robonomics-runtime
cargo build -p robonomics-runtime
cargo build -p robonomics-runtime-subxt-api --features build-metadata
```

---

**Error**: `Invalid metadata magic sequence`

**Solution**: The metadata format may have changed. This is usually a bug - report it.

### Connection Errors

**Error**: `Connection refused`

**Solution**: Ensure the Robonomics node is running and accessible:
```bash
# Check if node is running
curl -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' \
     ws://127.0.0.1:9988
```

---

**Error**: `Request timeout`

**Solution**: The node might be overloaded or the network is slow:
```rust
// Increase timeout in client configuration
use subxt::config::Config;
let client = OnlineClient::<RobonomicsConfig>::from_url_with_timeout(
    "ws://127.0.0.1:9988",
    std::time::Duration::from_secs(60)
).await?;
```

## Advanced Usage

### Custom Extrinsic Parameters

```rust
use robonomics_runtime_subxt_api::{RobonomicsExtrinsicParamsBuilder, RobonomicsConfig};
use subxt::config::polkadot::PlainTip;

// Build custom extrinsic params
let params = RobonomicsExtrinsicParamsBuilder::<RobonomicsConfig>::new()
    .tip(PlainTip::new(1_000_000))  // Add a tip
    .build();

// Use with transaction
client.tx()
    .await?
    .sign_and_submit(&tx, &signer, params)
    .await?;
```

### Offline Signing

```rust
use robonomics_runtime_subxt_api::{api, RobonomicsConfig};
use subxt::tx::TxPayload;

// Create transaction payload (offline)
let tx = api::tx().system().remark(vec![1, 2, 3]);

// Sign offline
let signed = client.tx()
    .await?
    .create_signed(&tx, &alice, Default::default())
    .await?;

// Submit later (online)
let hash = signed.submit().await?;
```

### Batch Transactions

```rust
use robonomics_runtime_subxt_api::api;

// Create multiple calls
let call1 = api::tx().system().remark(vec![1]);
let call2 = api::tx().system().remark(vec![2]);

// Batch them
let batch = api::tx().utility().batch(vec![call1, call2]);

client.tx().await?.sign_and_submit_default(&batch, &alice).await?;
```

## Examples

See the following projects for real-world usage:

- **libcps**: CPS pallet interaction library ([tools/libcps](../../../tools/libcps))
- **robonet**: Integration testing tool ([tools/robonet](../../../tools/robonet))

## Related Documentation

- [Subxt Documentation](https://docs.rs/subxt)
- [Polkadot SDK](https://paritytech.github.io/polkadot-sdk)

## License

Apache-2.0 - See [LICENSE](../../../LICENSE) for details.
