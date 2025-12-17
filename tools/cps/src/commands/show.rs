///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2025 Robonomics Network <research@robonomics.network>
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//
///////////////////////////////////////////////////////////////////////////////
//! Show command implementation.

use crate::display;
use anyhow::{anyhow, Result};
use libcps::blockchain::{Client, Config};
use libcps::crypto::{decrypt, KeypairType};
use libcps::node::Node;
use libcps::types::NodeData;
use sp_core::Pair;

pub async fn execute(config: &Config, node_id: u64, decrypt: bool, keypair_type: KeypairType) -> Result<()> {
    display::tree::progress("Connecting to blockchain...");

    let client = Client::new(config).await?;

    display::tree::info(&format!("Connected to {}", config.ws_url));
    display::tree::progress(&format!("Fetching node {node_id}..."));

    // Query node using Node API
    let node = Node::new(&client, node_id);
    let node_info = node.query().await?;

    // Try to decrypt if requested and data is encrypted
    let meta_str = if decrypt && node_info.meta.is_encrypted() {
        display::tree::info("🔓 Decrypting metadata...");
        match try_decrypt(&node_info.meta, config, keypair_type) {
            Ok(decrypted) => Some(String::from_utf8_lossy(&decrypted).to_string()),
            Err(e) => {
                display::tree::warning(&format!("Failed to decrypt metadata: {}", e));
                String::from_utf8(node_info.meta.as_bytes().to_vec()).ok()
            }
        }
    } else {
        String::from_utf8(node_info.meta.as_bytes().to_vec()).ok()
    };

    let payload_str = if decrypt && node_info.payload.is_encrypted() {
        display::tree::info("🔓 Decrypting payload...");
        match try_decrypt(&node_info.payload, config, keypair_type) {
            Ok(decrypted) => Some(String::from_utf8_lossy(&decrypted).to_string()),
            Err(e) => {
                display::tree::warning(&format!("Failed to decrypt payload: {}", e));
                String::from_utf8(node_info.payload.as_bytes().to_vec()).ok()
            }
        }
    } else {
        String::from_utf8(node_info.payload.as_bytes().to_vec()).ok()
    };

    display::tree::print_tree(
        node_id,
        &hex::encode(&node_info.owner),
        meta_str.as_deref(),
        payload_str.as_deref(),
        &node_info.children,
    );

    Ok(())
}

/// Helper function to decrypt data.
fn try_decrypt(
    data: &NodeData,
    config: &Config,
    keypair_type: KeypairType,
) -> Result<Vec<u8>> {
    if !data.is_encrypted() {
        return Err(anyhow!("Data is not encrypted"));
    }

    let suri = config
        .suri
        .as_ref()
        .ok_or_else(|| anyhow!("SURI required for decryption"))?;

    let encrypted_bytes = data.as_bytes();

    match keypair_type {
        KeypairType::Sr25519 => {
            let pair = sp_core::sr25519::Pair::from_string(suri, None)
                .map_err(|e| anyhow!("Failed to parse SR25519 keypair: {:?}", e))?;
            // Decrypt (sender verification disabled with None)
            decrypt(encrypted_bytes, &pair, None)
        }
        KeypairType::Ed25519 => {
            let pair = sp_core::ed25519::Pair::from_string(suri, None)
                .map_err(|e| anyhow!("Failed to parse ED25519 keypair: {:?}", e))?;
            // Decrypt (sender verification disabled with None)
            decrypt(encrypted_bytes, &pair, None)
        }
    }
}
