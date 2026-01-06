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

pub async fn execute(config: &Config, cipher: Option<&Cipher>, node_id: u64, decrypt: bool) -> Result<()> {
    display::tree::progress("Connecting to blockchain...");

    let client = Client::new(config).await?;

    display::tree::info(&format!("Connected to {}", config.ws_url));
    display::tree::progress(&format!("Fetching node {node_id}..."));

    // Query node using Node API
    let node = Node::new(&client, node_id);
    let node_info = node.query().await?;

    // Try to decrypt if requested and data is encrypted
    let meta_str = if decrypt && node_info.meta.is_encrypted() {
        let cipher = cipher.ok_or_else(|| anyhow::anyhow!("Cipher required for decryption"))?;
        display::tree::info("🔓 Decrypting metadata...");
        let decrypted = cipher.decrypt(node_info.meta.as_bytes(), None)
            .map_err(|e| anyhow::anyhow!("Failed to decrypt metadata: {}. Data appears to be encrypted but decryption failed.", e))?;
        Some(String::from_utf8_lossy(&decrypted).to_string())
    } else {
        String::from_utf8(node_info.meta.as_bytes().to_vec()).ok()
    };

    let payload_str = if decrypt && node_info.payload.is_encrypted() {
        let cipher = cipher.ok_or_else(|| anyhow::anyhow!("Cipher required for decryption"))?;
        display::tree::info("🔓 Decrypting payload...");
        let decrypted = cipher.decrypt(node_info.payload.as_bytes(), None)
            .map_err(|e| anyhow::anyhow!("Failed to decrypt payload: {}. Data appears to be encrypted but decryption failed.", e))?;
        Some(String::from_utf8_lossy(&decrypted).to_string())
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
