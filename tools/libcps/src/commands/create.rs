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
//! Create command implementation.

use crate::display;
use anyhow::Result;
use colored::*;
use libcps::blockchain::{Client, Config};
use libcps::crypto::Cipher;
use libcps::node::Node;
use libcps::types::NodeData;
use parity_scale_codec::Encode;
use subxt::utils::AccountId32;

pub async fn execute(
    config: &Config,
    cipher: Option<&Cipher>,
    parent: Option<u64>,
    meta: Option<String>,
    payload: Option<String>,
    receiver_public: Option<[u8; 32]>,
    algorithm: Option<libcps::crypto::EncryptionAlgorithm>,
) -> Result<()> {
    display::progress("Connecting to blockchain...");

    let client = Client::new(config).await?;
    let keypair = client.require_keypair()?;

    display::info(&format!("Connected to {}", config.ws_url));
    let account_id = AccountId32::from(keypair.public_key().0);
    display::info(&format!("Using account: {}", account_id));

    if parent.is_some() {
        display::info(&format!(
            "Creating child node under parent {}",
            parent.unwrap()
        ));
    } else {
        display::info("Creating root node");
    }

    // Convert strings to NodeData, applying encryption if requested
    let meta_data =
        if let (Some(receiver_pub), Some(ref m)) = (receiver_public.as_ref(), meta.as_ref()) {
            let cipher = cipher.ok_or_else(|| anyhow::anyhow!("Cipher required for encryption"))?;
            let algorithm =
                algorithm.ok_or_else(|| anyhow::anyhow!("Algorithm required for encryption"))?;
            display::info(&format!(
                "[E] Encrypting metadata with {} using {}",
                algorithm,
                cipher.scheme()
            ));
            let receiver_account = AccountId32::from(*receiver_pub);
            display::info(&format!(
                "[K] Receiver: {}",
                receiver_account
            ));

            let encrypted_message = cipher.encrypt(m.as_bytes(), receiver_pub, algorithm)?;
            let encrypted_bytes = encrypted_message.encode();
            Some(NodeData::aead_from(encrypted_bytes))
        } else {
            meta.map(|m| NodeData::from(m))
        };

    let payload_data =
        if let (Some(receiver_pub), Some(ref p)) = (receiver_public.as_ref(), payload.as_ref()) {
            let cipher = cipher.ok_or_else(|| anyhow::anyhow!("Cipher required for encryption"))?;
            let algorithm =
                algorithm.ok_or_else(|| anyhow::anyhow!("Algorithm required for encryption"))?;
            if meta_data.is_none() {
                display::info(&format!(
                    "[E] Encrypting payload with {} using {}",
                    algorithm,
                    cipher.scheme()
                ));
                let receiver_account = AccountId32::from(*receiver_pub);
                display::info(&format!(
                    "[K] Receiver: {}",
                    receiver_account
                ));
            }

            let encrypted_message = cipher.encrypt(p.as_bytes(), receiver_pub, algorithm)?;
            let encrypted_bytes = encrypted_message.encode();
            Some(NodeData::aead_from(encrypted_bytes))
        } else {
            payload.map(|p| NodeData::from(p))
        };

    let spinner = display::spinner("Submitting transaction...");
    let node = Node::create(&client, parent, meta_data, payload_data).await?;
    spinner.finish_and_clear();

    display::success(&format!(
        "Node created with ID: {}",
        node.id().to_string().bright_cyan()
    ));

    Ok(())
}
