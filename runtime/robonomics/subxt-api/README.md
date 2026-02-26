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

The build process extracts runtime metadata and generates type-safe APIs:

```
┌─────────────────────────────────────────────────────────────┐
│                      Build Process                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. build.rs loads runtime WASM from robonomics-runtime    │
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
│  ✓  Type-safe API ready to use                             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Why This Approach?

**Traditional Method:**
- Embed runtime WASM in subxt macro: `#[subxt::subxt(runtime_metadata_insecure_url = "...")]`
- Pull in all runtime dependencies (hundreds of crates)
- Slow compilation, large binary size

**Our Method:**
- Extract metadata once at build time
- Save to a file: `$OUT_DIR/metadata.scale`
- Reference in subxt macro: `runtime_metadata_path = "$OUT_DIR/metadata.scale"`
- **Result**: Fewer dependencies, faster builds, smaller binaries

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
    let account_info = client
        .storage()
        .at_latest()
        .await?
        .fetch(&api::storage().system().account(&alice))
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
        .sign_and_submit_default(&tx, &alice)
        .await?;
    
    println!("Transaction submitted with hash: {:?}", hash);
    
    Ok(())
}
```

### Working with CPS Pallet

```rust
use robonomics_runtime_subxt_api::{api, RobonomicsConfig, cps_impls::NodeData};
use subxt::OnlineClient;
use subxt_signer::sr25519::dev;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OnlineClient::<RobonomicsConfig>::from_url("ws://127.0.0.1:9988").await?;
    let alice = dev::alice();
    
    // Create a CPS node with plain data
    let node_data = NodeData::from("Hello from CPS!");
    let create_tx = api::tx().cps().create(None, node_data);
    
    let result = client
        .tx()
        .sign_and_submit_then_watch_default(&create_tx, &alice)
        .await?
        .wait_for_finalized_success()
        .await?;
    
    println!("Node created in block: {:?}", result.block_hash());
    
    // Query the node
    let node_id = 0; // Your node ID
    let node = client
        .storage()
        .at_latest()
        .await?
        .fetch(&api::storage().cps().nodes(node_id))
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
    let mut blocks = client.blocks().subscribe_finalized().await?;
    
    while let Some(block) = blocks.next().await {
        let block = block?;
        println!("Block #{}", block.number());
        
        // Process events in this block
        let events = block.events().await?;
        for event in events.iter() {
            let event = event?;
            println!("  Event: {}::{}", 
                event.pallet_name(), 
                event.variant_name()
            );
            
            // Handle specific events
            if let Ok(transfer) = event.as_event::<api::balances::events::Transfer>() {
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

// Storage queries
api::storage().system().account(account_id);
api::storage().cps().nodes(node_id);
api::storage().claim().claims(eth_address);

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

The `RobonomicsConfig` type is pre-configured for Robonomics nodes:

```rust
pub enum RobonomicsConfig {}

impl subxt::Config for RobonomicsConfig {
    type AccountId = AccountId32;           // Standard SS58 accounts
    type Signature = MultiSignature;        // Supports multiple signature types
    type Hasher = BlakeTwo256;              // Blake2b hashing
    type Header = SubstrateHeader<u32>;     // Standard Substrate header
    type AssetId = u32;                     // Asset ID type
    type Address = MultiAddress<AccountId32, ()>;  // Address format
    type ExtrinsicParams = RobonomicsExtrinsicParams<Self>;
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

## Build Requirements

### Dependencies

**Runtime Dependencies:**
- `subxt` - Substrate RPC client and type generator
- `parity-scale-codec` - SCALE encoding/decoding

**Build Dependencies:**
- `robonomics-runtime` - The runtime to extract metadata from
- `sc-executor` - WASM executor for metadata extraction
- `sp-state-machine` - Basic externalities for execution
- `sp-maybe-compressed-blob` - WASM decompression
- `sp-io` - Host functions for WASM execution

### Build Process

The build script (`build.rs`) performs these steps:

1. **Load Runtime**: Gets `WASM_BINARY` from `robonomics-runtime::dev`
2. **Decompress**: Handles compressed WASM blobs
3. **Create Executor**: Sets up WASM execution environment
4. **Extract Metadata**: Calls `Metadata_metadata` host function
5. **Validate**: Checks metadata magic bytes (`[0x6d, 0x65, 0x74, 0x61]`)
6. **Save**: Writes to `$OUT_DIR/metadata.scale`

### Rebuild Triggers

The build script automatically rebuilds when:
- Runtime source changes (`../src`)
- Runtime Cargo.toml changes (`../Cargo.toml`)
- Any dependency updates

## Troubleshooting

### Build Errors

**Error**: `WASM_BINARY is not available`

**Solution**: Ensure `robonomics-runtime` builds successfully first:
```bash
cargo build -p robonomics-runtime
cargo build -p robonomics-runtime-subxt-api
```

---

**Error**: `Unable to create RuntimeBlob from WASM`

**Solution**: The runtime WASM may be corrupted. Clean and rebuild:
```bash
cargo clean -p robonomics-runtime
cargo build -p robonomics-runtime-subxt-api
```

---

**Error**: `Invalid metadata magic sequence`

**Solution**: The metadata format may have changed. This is usually a bug - report it.

### Runtime Errors

**Error**: `Metadata hash mismatch`

**Solution**: The runtime on the node doesn't match your compiled metadata. Ensure:
1. You're connecting to a node with matching runtime version
2. Your `robonomics-runtime` dependency matches the node's runtime
3. Rebuild if you updated runtime dependencies

---

**Error**: `Call not found` or `Storage item not found`

**Solution**: The pallet or call might not exist in this runtime version:
1. Check the runtime includes the pallet
2. Verify the pallet name and call/storage name are correct
3. Check if the feature was added in a newer runtime version

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
    .create_signed(&tx, &alice, Default::default())
    .await?;

// Submit later (online)
let hash = client.tx().submit(signed).await?;
```

### Batch Transactions

```rust
use robonomics_runtime_subxt_api::api;

// Create multiple calls
let call1 = api::tx().system().remark(vec![1]);
let call2 = api::tx().system().remark(vec![2]);

// Batch them
let batch = api::tx().utility().batch(vec![call1, call2]);

client.tx().sign_and_submit_default(&batch, &alice).await?;
```

## Examples

See the following projects for real-world usage:

- **libcps**: CPS pallet interaction library ([tools/libcps](../../tools/libcps))
- **robonet**: Integration testing tool ([tools/robonet](../../tools/robonet))

## Related Documentation

- [Subxt Documentation](https://docs.rs/subxt)
- [Robonomics Runtime](../README.md)
- [Polkadot SDK](https://paritytech.github.io/polkadot-sdk)

## License

Apache-2.0 - See [LICENSE](../../../LICENSE) for details.
