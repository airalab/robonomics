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
//! Set payload command implementation (CLI interface).
//!
//! This module provides the CLI command wrapper for the library's node operations.
//! It handles display formatting and user interaction while delegating business logic to the
//! node module.

use crate::display;
use anyhow::Result;
use colored::*;
use libcps::blockchain::{Client, Config};
use libcps::crypto::Cipher;
use libcps::node::Node;
use libcps::types::NodeData;

pub async fn execute(
    config: &Config,
    cipher: Option<&Cipher>,
    node_id: u64,
    data: String,
    receiver_public: Option<[u8; 32]>,
) -> Result<()> {
    // CLI display: show connection progress
    display::tree::progress("Connecting to blockchain...");

    let client = Client::new(config).await?;
    let _keypair = client.require_keypair()?;

    display::tree::info(&format!("Connected to {}", config.ws_url));
    display::tree::info(&format!("Updating payload for node {node_id}"));

    // Convert data to NodeData, applying encryption if requested
    let payload_data = if let Some(receiver_pub) = receiver_public.as_ref() {
        let cipher = cipher.ok_or_else(|| anyhow::anyhow!("Cipher required for encryption"))?;
        display::tree::info(&format!(
            "🔐 Encrypting payload with {} using {}",
            cipher.algorithm(),
            cipher.scheme()
        ));
        display::tree::info(&format!("🔑 Receiver: {}", hex::encode(receiver_pub)));

        let encrypted_bytes = cipher.encrypt(data.as_bytes(), receiver_pub)?;
        NodeData::aead_from(encrypted_bytes)
    } else {
        NodeData::from(data)
    };

    // Create a Node handle and delegate to node operation (business logic)
    let node = Node::new(&client, node_id);

    let _events = node.set_payload(Some(payload_data)).await?;

    display::tree::success(&format!(
        "Payload updated for node {}",
        node_id.to_string().bright_cyan()
    ));

    Ok(())
}
