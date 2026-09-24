# Robonomics Runtime Subxt API

A type-safe API generator for the Robonomics runtime that generates compile-time verified blockchain interactions using [subxt](https://github.com/paritytech/subxt).

## Overview

This crate provides:
- **Type-safe API generation** using subxt's macro system
- **Minimal dependencies** compared to embedding runtime WASM directly
- **Always synchronized** metadata that matches your runtime version
- **Custom configuration** (`RobonomicsConfig`) tailored for Robonomics nodes

Runtime metadata is no longer owned by this crate. Obtaining, validating, and
exposing the SCALE-encoded Robonomics runtime metadata is the responsibility
of the [`robonomics-runtime-metadata`](../metadata) crate, which acts as the
single source of truth for the metadata. This crate depends on it and simply
copies the exported `robonomics_runtime_metadata::METADATA` bytes into its own
`$OUT_DIR` for the `subxt::subxt` macro to consume. See that crate's
[README](../metadata/README.md) for full details on how metadata is obtained,
validated, and updated.

## How It Works

```
┌─────────────────────────────────────────────────────────────┐
│                     subxt-api build                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. robonomics-runtime-metadata resolves METADATA           │
│     ↓                                                       │
│  2. build.rs writes METADATA to $OUT_DIR/metadata.scale     │
│     ↓                                                       │
│  3. subxt macro reads metadata and generates types          │
│     ↓                                                       │
│  ✓  Type-safe API ready to use                              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Why This Approach?

**Traditional Method:**
- Embed runtime WASM in subxt macro: `#[subxt::subxt(runtime_metadata_insecure_url = "...")]`
- Pull in all runtime dependencies (hundreds of crates)
- Slow compilation, large binary size

**Our Method:**
- Delegate metadata extraction/validation to `robonomics-runtime-metadata`
- Use prebuilt metadata by default: **instant builds**, no runtime dependencies
- Reference in subxt macro: `runtime_metadata_path = "$OUT_DIR/metadata.scale"`
- **Result**: Fewer dependencies, faster builds, smaller binaries, and a
  single source of truth shared with other API generators (e.g. a future
  C++/embedded generator)

## Build Features

This crate preserves its previous public feature interface by forwarding the
features to `robonomics-runtime-metadata`:

```toml
[features]
build-metadata = ["robonomics-runtime-metadata/build-metadata"]
```

### Default Build (No Features)

The fastest option - uses the metadata committed to `robonomics-runtime-metadata`:

```bash
# Fast build using prebuilt metadata
cargo build -p robonomics-runtime-subxt-api
```

**When to use:**
- Normal development and testing
- CI/CD pipelines where speed matters
- When runtime hasn't changed

### `build-metadata` Feature

Extracts fresh metadata from the runtime WASM (delegated to `robonomics-runtime-metadata`):

```bash
# Build with metadata extraction
cargo build -p robonomics-runtime-subxt-api --features build-metadata
```

**When to use:**
- After modifying runtime code
- When you need to update the prebuilt metadata.scale
- To ensure metadata is in sync with runtime

## Updating Prebuilt Metadata

The prebuilt `metadata.scale` file, and the procedure to regenerate it, now
live in the [`robonomics-runtime-metadata`](../metadata) crate. See its
[README](../metadata/README.md#updating-prebuilt-metadata) for the up to date
procedure.

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

    // Almost all interactions happen in the context of a specific block.
    // `at_current_block()` picks the current best block at the time of the call.
    let at_block = client.at_current_block().await?;

    // Query chain state - e.g., get system account info
    let alice: AccountId32 = subxt_signer::sr25519::dev::alice().public_key().into();
    let account_info = at_block
        .storage()
        .fetch(api::storage().system().account(), (alice,))
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
    let tx = api::transactions().system().remark(remark);

    // `client.tx()` is a shorthand for `client.at_current_block().await?.transactions()`.
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
    let create_tx = api::transactions().cps().create(None, node_data.into());

    let result = client
        .tx()
        .await?
        .sign_and_submit_then_watch_default(&create_tx, &alice)
        .await?
        .wait_for_finalized_success()
        .await?;

    println!("Node created in block: {:?}", result.extrinsic_hash());

    // Query the node at the current block
    let node_id = 0; // Your node ID
    let at_block = client.at_current_block().await?;
    let node = at_block
        .storage()
        .fetch(api::storage().cps().nodes(), (node_id,))
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

    // Stream finalized blocks
    let mut blocks = client.stream_blocks().await?;

    while let Some(block) = blocks.next().await {
        let block = block?;
        println!("Block #{}", block.number());

        // Move to a client scoped to this block to fetch its extrinsics/events.
        let at_block = block.at().await?;
        let extrinsics = at_block.extrinsics().fetch().await?;

        for ext in extrinsics.iter() {
            let ext = ext?;
            let events = ext.events().await?;

            for event in events.iter() {
                let event = event?;
                println!("  Event: {}::{}", event.pallet_name(), event.event_name());
            }

            // Handle a specific event type
            if let Some(Ok(transfer)) = events.find_first::<api::balances::events::Transfer>() {
                println!(
                    "    Transfer: {:?} -> {:?}, amount: {:?}",
                    transfer.from, transfer.to, transfer.amount
                );
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

// Storage addresses - fetched via `at_block.storage().fetch(address, keys)`
api::storage().system().account();
api::storage().cps().nodes();

// Transactions
api::transactions().system().remark(data);
api::transactions().cps().create(parent, data);
api::transactions().balances().transfer_allow_death(dest, value);

// Constants - read via `at_block.constants().entry(address)`
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
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct RobonomicsConfig;

impl subxt::Config for RobonomicsConfig {
    type AccountId = <SubstrateConfig as subxt::Config>::AccountId;   // Standard SS58 accounts
    type Signature = <SubstrateConfig as subxt::Config>::Signature;   // Supports multiple signature types
    type Hasher = <SubstrateConfig as subxt::Config>::Hasher;
    type Header = <SubstrateConfig as subxt::Config>::Header;
    type AssetId = <SubstrateConfig as subxt::Config>::AssetId;
    type Address = MultiAddress<Self::AccountId, ()>;                 // Address format
    type TransactionExtensions = RobonomicsTransactionExtensions<Self>;
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

Errors like `Metadata hash mismatch`, `WASM_BINARY is not available`,
`Unable to create RuntimeBlob from WASM`, and `Invalid metadata magic
sequence` all originate from the metadata extraction/validation logic that
now lives in `robonomics-runtime-metadata`. See that crate's
[README troubleshooting section](../metadata/README.md#troubleshooting) for
solutions.

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

**Solution**: `OnlineClient` doesn't expose a timeout knob directly. If the node is overloaded or
the network is slow, either wrap calls with `tokio::time::timeout`, or supply a custom RPC client
(e.g. `ReconnectingRpcClient`) configured with its own timeout/retry behavior:

```rust
use robonomics_runtime_subxt_api::RobonomicsConfig;
use subxt::OnlineClient;
use subxt_rpcs::client::{ReconnectingRpcClient, RpcClient};

let inner_rpc_client = ReconnectingRpcClient::builder()
    .build("ws://127.0.0.1:9988")
    .await?;
let rpc_client = RpcClient::new(inner_rpc_client);
let client = OnlineClient::<RobonomicsConfig>::from_rpc_client(rpc_client).await?;
```

## Advanced Usage

### Custom Extrinsic Parameters

```rust
use robonomics_runtime_subxt_api::{api, RobonomicsConfig};
use subxt::config::DefaultExtrinsicParamsBuilder;

// Build custom extrinsic params
let params = DefaultExtrinsicParamsBuilder::<RobonomicsConfig>::new()
    .tip(1_000_000)  // Add a tip
    .build();

// Use with transaction
let tx = api::transactions().system().remark(vec![1, 2, 3]);
client.tx()
    .await?
    .sign_and_submit(&tx, &signer, params)
    .await?;
```

### Offline Signing

```rust
use robonomics_runtime_subxt_api::{api, RobonomicsConfig};

// Create transaction payload (offline)
let tx = api::transactions().system().remark(vec![1, 2, 3]);

// Sign offline
let mut txs = client.tx().await?;
let signed = txs
    .create_signed(&tx, &alice, Default::default())
    .await?;

// Submit later (online)
let hash = signed.submit().await?;
```

### Batch Transactions

```rust
use robonomics_runtime_subxt_api::api;

// Create multiple calls
let call1 = api::transactions().system().remark(vec![1]);
let call2 = api::transactions().system().remark(vec![2]);

// Batch them
let batch = api::transactions().utility().batch(vec![call1, call2]);

client.tx().await?.sign_and_submit_default(&batch, &alice).await?;
```

## Examples

See the following projects for real-world usage:

- **libcps**: CPS pallet interaction library ([airalab/robins](https://github.com/airalab/robins))

## Related Documentation

- [Subxt Documentation](https://docs.rs/subxt)
- [Polkadot SDK](https://paritytech.github.io/polkadot-sdk)

## License

Apache-2.0 - See [LICENSE](../../../LICENSE) for details.
