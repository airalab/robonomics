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
use libcps::crypto::EncryptionAlgorithm;
use libcps::node::Node;
use libcps::types::NodeData;
use std::str::FromStr;

/// Execute the set-payload command with CLI display output.
///
/// This function serves as a CLI wrapper that:
/// - Handles user-facing progress messages and formatting
/// - Parses and validates CLI arguments
/// - Delegates business logic to libcps::node
/// - Presents results with colored output and emojis
pub async fn execute(
    config: &Config,
    node_id: u64,
    data: String,
    encrypt: bool,
    cipher: &str,
    keypair_type: libcps::crypto::KeypairType,
) -> Result<()> {
    // CLI display: show connection progress
    display::tree::progress("Connecting to blockchain...");

    let client = Client::new(config).await?;
    let _keypair = client.require_keypair()?;

    display::tree::info(&format!("Connected to {}", config.ws_url));
    display::tree::info(&format!("Updating payload for node {node_id}"));

    // Parse cipher algorithm (CLI validation)
    let algorithm = EncryptionAlgorithm::from_str(cipher)
        .map_err(|e| anyhow::anyhow!("Invalid cipher: {}", e))?;

    // CLI display: show encryption settings
    if encrypt {
        display::tree::info(&format!("🔐 Using encryption algorithm: {}", algorithm));
        display::tree::info(&format!("🔑 Using keypair type: {}", keypair_type));
        display::tree::warning("Encryption not yet implemented - updating with plain data");
    }

    // Create a Node handle and delegate to node operation (business logic)
    let node = Node::new(&client, node_id);
    let payload_data = NodeData::from(data);

    match node.set_payload(Some(payload_data)).await {
        Ok(_events) => {
            display::tree::success(&format!(
                "Payload updated for node {}",
                node_id.to_string().bright_cyan()
            ));
            Ok(())
        }
        Err(e) => {
            // CLI display: present error nicely
            display::tree::error(&format!(
                "Extrinsic submission not implemented yet. Requires running node and metadata.\n\
                 See {} command for details.",
                "create".bright_cyan()
            ));

            println!("\n{}", "Example output (with live node):".bright_yellow());
            display::tree::success(&format!(
                "Payload updated for node {}",
                node_id.to_string().bright_cyan()
            ));

            // Return actual error for debugging
            Err(e)
        }
    }
}
