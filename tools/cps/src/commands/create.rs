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

use libcps::blockchain::{Client, Config};
use libcps::node::{Node, CreateNodeParams};
use crate::display;
use libcps::crypto::EncryptionAlgorithm;
use anyhow::Result;
use colored::*;
use std::str::FromStr;

pub async fn execute(
    config: &Config,
    parent: Option<u64>,
    meta: Option<String>,
    payload: Option<String>,
    encrypt: bool,
    cipher: &str,
    keypair_type: libcps::crypto::KeypairType,
) -> Result<()> {
    display::tree::progress("Connecting to blockchain...");
    
    let client = Client::new(config).await?;
    let keypair = client.require_keypair()?;

    display::tree::info(&format!("Connected to {}", config.ws_url));
    display::tree::info(&format!("Using account: {}", hex::encode(keypair.public_key().0)));

    // Parse cipher algorithm
    let algorithm = EncryptionAlgorithm::from_str(cipher)
        .map_err(|e| anyhow::anyhow!("Invalid cipher: {}", e))?;

    if encrypt {
        display::tree::info(&format!("🔐 Using encryption algorithm: {}", algorithm));
        display::tree::info(&format!("🔑 Using keypair type: {}", keypair_type));
    }

    if parent.is_some() {
        display::tree::info(&format!("Creating child node under parent {}", parent.unwrap()));
    } else {
        display::tree::info("Creating root node");
    }

    // Create node using Node API
    let params = CreateNodeParams {
        parent,
        meta: meta.map(|m| m.into_bytes()),
        payload: payload.map(|p| p.into_bytes()),
        encrypt,
        algorithm,
        keypair_type,
        recipient_public: None, // TODO: Add CLI option for recipient
    };

    display::tree::progress("Creating node...");
    let result = Node::create(&client, params).await?;

    display::tree::success(&format!("Node created with ID: {}", result.node_id.to_string().bright_cyan()));

    Ok(())
}
