# CPS Library Implementation Guide

This guide explains how to complete the implementation of the CPS library by wiring up the blockchain interactions, encryption, and MQTT bridge.

## Current Status

The following components are **COMPLETE**:
- ✅ Generated subxt runtime metadata (`src/robonomics_runtime.rs`)
- ✅ Crypto module with encryption/decryption (`src/crypto/`)
- ✅ Display utilities for CLI output (`src/display/`)
- ✅ Type definitions (`src/types.rs`)
- ✅ Blockchain client (`src/blockchain/client.rs`)
- ✅ Command structure and CLI parsing (`src/main.rs`, `src/commands/mod.rs`)

The following components need **IMPLEMENTATION**:
- ❌ Node operations in `src/node.rs` (6 functions)
- ❌ Encryption wiring in commands
- ❌ Decryption in show command
- ❌ MQTT bridge functionality

## Prerequisites

Before implementing, you need:

1. **A running Robonomics node** with the CPS pallet:
   ```bash
   robonomics --dev --tmp
   ```

2. **Test the node** to verify CPS pallet is available:
   ```bash
   # Should show CPS in the list
   subxt metadata --url ws://localhost:9944 | grep -i cps
   ```

3. **Regenerate metadata** if needed:
   ```bash
   cd tools/cps
   subxt metadata --url ws://localhost:9944 > metadata.scale
   subxt codegen --file metadata.scale > src/robonomics_runtime.rs
   ```

## Implementation Steps

### Step 1: Find CPS Pallet API in Generated Metadata

The generated file `src/robonomics_runtime.rs` contains the CPS pallet API. Look for:

```rust
pub mod cps {
    // Transaction calls
    pub fn create_node(...) -> ...
    pub fn set_meta(...) -> ...
    pub fn set_payload(...) -> ...
    pub fn move_node(...) -> ...
    pub fn delete_node(...) -> ...
}

// Storage queries
pub mod storage {
    pub mod cps {
        pub fn nodes(...) -> ...
        pub fn nodes_by_parent(...) -> ...
        pub fn root_nodes() -> ...
    }
}
```

### Step 2: Implement Node Operations

Edit `src/node.rs` and replace the TODO implementations:

#### 2.1 Implement `create()`

```rust
pub async fn create(
    client: &'a Client,
    parent: Option<u64>,
    meta: Option<NodeData>,
    payload: Option<NodeData>,
) -> Result<Self> {
    use crate::robonomics_runtime;
    
    let keypair = client.require_keypair()?;
    
    // Build the create_node call
    let create_call = robonomics_runtime::tx().cps().create_node(
        parent.map(|p| crate::types::NodeId(p)),
        meta,
        payload,
    );
    
    // Submit and watch the transaction
    let events = client
        .api
        .tx()
        .sign_and_submit_then_watch_default(&create_call, keypair)
        .await?
        .wait_for_finalized_success()
        .await?;
    
    // Extract the created node ID from events
    // Look for CPS::NodeCreated event
    let node_id = events
        .find_first::<robonomics_runtime::cps::events::NodeCreated>()?
        .ok_or_else(|| anyhow!("NodeCreated event not found"))?
        .node_id
        .0;
    
    Ok(Self {
        client,
        id: node_id,
    })
}
```

#### 2.2 Implement `query()`

```rust
pub async fn query(&self) -> Result<NodeInfo> {
    use crate::robonomics_runtime;
    
    // Query the node from storage
    let nodes_query = robonomics_runtime::storage().cps().nodes(
        crate::types::NodeId(self.id)
    );
    
    let node = self
        .client
        .api
        .storage()
        .at_latest()
        .await?
        .fetch(&nodes_query)
        .await?
        .ok_or_else(|| anyhow!("Node not found: {}", self.id))?;
    
    // Query children
    let children_query = robonomics_runtime::storage().cps().nodes_by_parent(
        crate::types::NodeId(self.id)
    );
    
    let children = self
        .client
        .api
        .storage()
        .at_latest()
        .await?
        .fetch(&children_query)
        .await?
        .unwrap_or_default()
        .iter()
        .map(|id| id.0)
        .collect();
    
    Ok(NodeInfo {
        id: self.id,
        owner: node.owner.into(),
        parent: node.parent.map(|p| p.0),
        meta: node.meta.unwrap_or_else(|| NodeData::Plain(vec![])),
        payload: node.payload.unwrap_or_else(|| NodeData::Plain(vec![])),
        children,
    })
}
```

#### 2.3 Implement `set_meta()`, `set_payload()`, `move_to()`, `delete()`

Follow similar patterns:
1. Build the extrinsic call using `robonomics_runtime::tx().cps()...`
2. Sign and submit with `client.api.tx().sign_and_submit_then_watch_default()`
3. Wait for finalization with `.wait_for_finalized_success()`
4. Return the events

### Step 3: Wire Up Encryption in Commands

#### 3.1 Update `src/commands/create.rs`

Replace the encryption warning section:

```rust
// Before sending, encrypt if needed
let final_meta_data = if encrypt {
    match meta_data {
        Some(plain_data) => {
            let bytes = plain_data.as_bytes();
            
            // Get receiver public key (node owner)
            let receiver_public = keypair.public_key().0;
            
            // Encrypt based on keypair type
            let encrypted_bytes = match keypair_type {
                libcps::crypto::KeypairType::Sr25519 => {
                    let secret = /* Convert keypair to secret */;
                    libcps::crypto::encrypt(
                        bytes,
                        &secret,
                        &receiver_public,
                        algorithm,
                    )?
                },
                libcps::crypto::KeypairType::Ed25519 => {
                    /* Use Ed25519 encryption */
                    unimplemented!("Ed25519 encryption conversion from sr25519 keypair")
                },
            };
            
            // Wrap in appropriate EncryptedData variant
            let encrypted_data = match algorithm {
                EncryptionAlgorithm::XChaCha20Poly1305 => {
                    crate::types::EncryptedData::XChaCha20Poly1305(encrypted_bytes)
                },
                EncryptionAlgorithm::AesGcm256 => {
                    crate::types::EncryptedData::AesGcm256(encrypted_bytes)
                },
                EncryptionAlgorithm::ChaCha20Poly1305 => {
                    crate::types::EncryptedData::ChaCha20Poly1305(encrypted_bytes)
                },
            };
            
            Some(NodeData::Encrypted(encrypted_data))
        },
        None => None,
    }
} else {
    meta_data
};

// Same for payload_data...
```

### Step 4: Implement Decryption in `show` Command

Update `src/commands/show.rs`:

```rust
if decrypt && node_info.payload.is_encrypted() {
    match try_decrypt(&node_info.payload, keypair) {
        Ok(decrypted) => {
            display::tree::info(&format!("Decrypted payload: {}", 
                String::from_utf8_lossy(&decrypted)
            ));
        },
        Err(e) => {
            display::tree::warning(&format!("Failed to decrypt: {}", e));
        }
    }
}

fn try_decrypt(data: &NodeData, keypair: &Keypair) -> Result<Vec<u8>> {
    match data {
        NodeData::Encrypted(encrypted) => {
            let bytes = encrypted.as_bytes();
            let secret = /* Convert keypair to secret */;
            libcps::crypto::decrypt(bytes, &secret, None)
        },
        _ => Err(anyhow!("Data is not encrypted")),
    }
}
```

### Step 5: Implement MQTT Bridge

Update `src/commands/mqtt.rs`:

#### 5.1 MQTT Subscribe

```rust
pub async fn subscribe(...) -> Result<()> {
    use rumqttc::{AsyncClient, MqttOptions, QoS};
    
    let client = Client::new(blockchain_config).await?;
    let keypair = client.require_keypair()?;
    
    // Setup MQTT client
    let mut mqttoptions = MqttOptions::new(
        mqtt_config.client_id.clone().unwrap_or_else(|| format!("cps-sub-{}", node_id)),
        &mqtt_config.broker,
        1883,
    );
    
    if let Some(username) = &mqtt_config.username {
        mqttoptions.set_credentials(username, mqtt_config.password.as_deref().unwrap_or(""));
    }
    
    let (mqtt_client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    mqtt_client.subscribe(topic, QoS::AtMostOnce).await?;
    
    display::tree::success(&format!("Subscribed to {}", topic));
    
    // Event loop
    loop {
        let notification = eventloop.poll().await?;
        if let rumqttc::Event::Incoming(rumqttc::Packet::Publish(p)) = notification {
            let data = p.payload.to_vec();
            
            // Encrypt if needed
            let payload_data = if encrypt {
                /* Encrypt data */
                unimplemented!("Encrypt before storing")
            } else {
                NodeData::Plain(data)
            };
            
            // Submit to blockchain
            let node = Node::new(&client, node_id);
            node.set_payload(Some(payload_data)).await?;
            
            display::tree::success(&format!("Updated node {} with MQTT data", node_id));
        }
    }
}
```

#### 5.2 MQTT Publish

Similar pattern but poll blockchain and publish changes to MQTT.

## Testing

Once implemented, test with:

```bash
# Terminal 1: Start dev node
robonomics --dev --tmp

# Terminal 2: Set environment
export ROBONOMICS_WS_URL=ws://localhost:9944
export ROBONOMICS_SURI=//Alice

# Test create
cargo run --bin cps -- create --meta '{"test":true}'

# Test show
cargo run --bin cps -- show 0

# Test with encryption
cargo run --bin cps -- create --parent 0 --payload "secret" --encrypt

# Test decryption
cargo run --bin cps -- show 1 --decrypt

# Test MQTT (requires mosquitto)
# Terminal 3:
mosquitto -v

# Terminal 2:
cargo run --bin cps -- mqtt subscribe "test/topic" 1

# Terminal 4:
mosquitto_pub -t "test/topic" -m "hello"
```

## Troubleshooting

### Issue: "CPS pallet not found in metadata"

The CPS pallet might not be enabled in your node. Check:
- Runtime configuration includes CPS pallet
- Node binary is built with the pallet
- Metadata generation is from the correct node

### Issue: "Type mismatch" errors

The types in `src/types.rs` must exactly match the pallet types:
- Check the pallet's `Config` trait
- Verify `NodeData` and `EncryptedData` enum variants match
- Ensure `NodeId` uses correct encoding

### Issue: Encryption key conversion

The subxt `Keypair` type may not directly convert to crypto library types. You may need:
```rust
use sp_core::sr25519::Pair as Sr25519Pair;
use sp_core::Pair;

// Convert subxt keypair to schnorrkel secret
let pair = Sr25519Pair::from_string(suri, None)?;
let secret_key = pair.as_ref().secret;
```

## References

- [Subxt Documentation](https://docs.rs/subxt/)
- [CPS Pallet Source](../../frame/cps/src/lib.rs)
- [Encryption Module](./src/crypto/)
- [MQTT Client (rumqttc)](https://docs.rs/rumqttc/)
