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
use anyhow::Result;
use libcps::blockchain::{Client, Config};
use libcps::crypto::Cipher;
use libcps::node::Node;
use libcps::types::{EncryptedData, NodeData};

pub async fn execute(
    config: &Config,
    cipher: Option<&Cipher>,
    node_id: u64,
    decrypt: bool,
) -> Result<()> {
    display::tree::progress("Connecting to blockchain...");

    let client = Client::new(config).await?;

    display::tree::info(&format!("Connected to {}", config.ws_url));
    display::tree::progress(&format!("Fetching node {node_id}..."));

    // Query node using Node API
    let node = Node::new(&client, node_id);
    let node_info = node.query().await?;

    // Helper function to extract bytes from NodeData
    fn extract_bytes(node_data: &libcps::types::NodeData) -> Vec<u8> {
        match node_data {
            NodeData::Plain(bounded_vec) => bounded_vec.0.clone(),
            NodeData::Encrypted(EncryptedData::Aead(bounded_vec)) => bounded_vec.0.clone(),
        }
    }

    // Helper function to check if NodeData is encrypted
    fn is_encrypted(node_data: &libcps::types::NodeData) -> bool {
        matches!(node_data, libcps::types::NodeData::Encrypted(_))
    }

    // Try to decrypt if requested and data is encrypted
    let meta_str = if let Some(ref meta) = node_info.meta {
        if decrypt && is_encrypted(meta) {
            let cipher = cipher.ok_or_else(|| anyhow::anyhow!("Cipher required for decryption"))?;
            display::tree::info("🔓 Decrypting metadata...");
            let bytes = extract_bytes(meta);
            let message: libcps::crypto::EncryptedMessage = serde_json::from_slice(&bytes)
                .map_err(|e| anyhow::anyhow!("Failed to parse encrypted metadata: {}", e))?;
            let decrypted = cipher.decrypt(&message, None)
                .map_err(|e| anyhow::anyhow!("Failed to decrypt metadata: {}. Data appears to be encrypted but decryption failed.", e))?;
            Some(String::from_utf8_lossy(&decrypted).to_string())
        } else {
            let bytes = extract_bytes(meta);
            String::from_utf8(bytes).ok()
        }
    } else {
        None
    };

    let payload_str = if let Some(ref payload) = node_info.payload {
        if decrypt && is_encrypted(payload) {
            let cipher = cipher.ok_or_else(|| anyhow::anyhow!("Cipher required for decryption"))?;
            display::tree::info("🔓 Decrypting payload...");
            let bytes = extract_bytes(payload);
            let message: libcps::crypto::EncryptedMessage = serde_json::from_slice(&bytes)
                .map_err(|e| anyhow::anyhow!("Failed to parse encrypted payload: {}", e))?;
            let decrypted = cipher.decrypt(&message, None)
                .map_err(|e| anyhow::anyhow!("Failed to decrypt payload: {}. Data appears to be encrypted but decryption failed.", e))?;
            Some(String::from_utf8_lossy(&decrypted).to_string())
        } else {
            let bytes = extract_bytes(payload);
            String::from_utf8(bytes).ok()
        }
    } else {
        None
    };

    display::tree::print_tree(
        node_id,
        node_info.owner,
        meta_str.as_deref(),
        payload_str.as_deref(),
        &node_info.children,
    );

    Ok(())
}
