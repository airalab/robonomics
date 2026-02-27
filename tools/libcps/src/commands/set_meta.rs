///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2026 Robonomics Network <research@robonomics.network>
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
use libcps::crypto::Cipher;
use libcps::node::{Node, NodeData};
use parity_scale_codec::Encode;
use subxt::utils::AccountId32;

pub async fn execute(
    config: &Config,
    cipher: Option<&Cipher>,
    node_id: u64,
    data: String,
    receiver_public: Option<[u8; 32]>,
    algorithm: Option<libcps::crypto::EncryptionAlgorithm>,
) -> Result<()> {
    display::progress("Connecting to blockchain...");

    let client = Client::new(config).await?;
    let _keypair = client.require_keypair()?;

    display::info(&format!("Connected to {}", config.ws_url));
    display::info(&format!("Updating metadata for node {node_id}"));

    // Convert data to NodeData, applying encryption if requested
    let meta_data = if let Some(receiver_pub) = receiver_public.as_ref() {
        let cipher = cipher.ok_or_else(|| anyhow::anyhow!("Cipher required for encryption"))?;
        let algorithm =
            algorithm.ok_or_else(|| anyhow::anyhow!("Algorithm required for encryption"))?;
        display::info(&format!(
            "[E] Encrypting metadata with {} using {}",
            algorithm,
            cipher.scheme()
        ));
        let receiver_account = AccountId32::from(*receiver_pub);
        display::info(&format!("[K] Receiver: {}", receiver_account));

        let encrypted_message = cipher.encrypt(data.as_bytes(), receiver_pub, algorithm)?;
        let encrypted_bytes = encrypted_message.encode();
        NodeData::aead_from(encrypted_bytes)
    } else {
        NodeData::from(data)
    };

    // Update metadata using Node API with NodeData
    let node = Node::new(&client, node_id);

    let spinner = display::spinner("Submitting transaction...");
    let _events = node.set_meta(Some(meta_data)).await?;
    spinner.finish_and_clear();

    display::success(&format!(
        "Metadata updated for node {}",
        node_id.to_string().bright_cyan()
    ));

    Ok(())
}
