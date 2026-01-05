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
//! Set metadata command implementation.

use crate::display;
use anyhow::Result;
use colored::*;
use libcps::blockchain::{Client, Config};
use libcps::crypto::Cypher;
use libcps::node::Node;
use libcps::types::NodeData;
use sp_core::Pair;

pub async fn execute(
    config: &Config,
    cypher: Option<&Cypher>,
    node_id: u64,
    data: String,
    encrypt: bool,
) -> Result<()> {
    display::tree::progress("Connecting to blockchain...");

    let client = Client::new(config).await?;
    let _keypair = client.require_keypair()?;

    display::tree::info(&format!("Connected to {}", config.ws_url));
    display::tree::info(&format!("Updating metadata for node {node_id}"));

    // Convert data to NodeData, applying encryption if requested
    let meta_data = if encrypt {
        let cypher = cypher.ok_or_else(|| anyhow::anyhow!("Cypher required for encryption"))?;
        display::tree::info(&format!("🔐 Encrypting metadata with {} using {}", cypher.algorithm(), cypher.scheme()));
        
        // Get own public key for encryption
        let own_public = match cypher.scheme() {
            libcps::crypto::CryptoScheme::Sr25519 => {
                let suri = config.suri.as_ref().ok_or_else(|| anyhow::anyhow!("SURI required"))?;
                let pair = sp_core::sr25519::Pair::from_string(suri, None)
                    .map_err(|e| anyhow::anyhow!("Failed to parse keypair: {:?}", e))?;
                pair.public().0.to_vec()
            }
            libcps::crypto::CryptoScheme::Ed25519 => {
                let suri = config.suri.as_ref().ok_or_else(|| anyhow::anyhow!("SURI required"))?;
                let pair = sp_core::ed25519::Pair::from_string(suri, None)
                    .map_err(|e| anyhow::anyhow!("Failed to parse keypair: {:?}", e))?;
                pair.public().0.to_vec()
            }
        };
        
        let encrypted_bytes = cypher.encrypt(data.as_bytes(), &own_public)?;
        NodeData::from_encrypted_bytes(encrypted_bytes, cypher.algorithm())
    } else {
        NodeData::from(data)
    };

    // Update metadata using Node API with NodeData
    let node = Node::new(&client, node_id);

    display::tree::progress("Updating metadata...");
    let _events = node.set_meta(Some(meta_data)).await?;

    display::tree::success(&format!(
        "Metadata updated for node {}",
        node_id.to_string().bright_cyan()
    ));

    Ok(())
}
