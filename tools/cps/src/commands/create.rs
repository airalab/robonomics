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
use crate::display;
use libcps::types::NodeData;
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

    // Prepare metadata
    let meta_data = if let Some(m) = meta {
        if encrypt {
            display::tree::warning("Encryption not yet fully implemented (requires recipient public key)");
            Some(NodeData::plain(m.as_bytes()))
        } else {
            Some(NodeData::plain(m.as_bytes()))
        }
    } else {
        None
    };

    // Prepare payload
    let payload_data = if let Some(p) = payload {
        if encrypt {
            display::tree::warning("Encryption not yet fully implemented (requires recipient public key)");
            Some(NodeData::plain(p.as_bytes()))
        } else {
            Some(NodeData::plain(p.as_bytes()))
        }
    } else {
        None
    };

    if parent.is_some() {
        display::tree::info(&format!("Creating child node under parent {}", parent.unwrap()));
    } else {
        display::tree::info("Creating root node");
    }

    // In a real implementation, we would submit the extrinsic here:
    //
    // #[subxt::subxt(runtime_metadata_path = "metadata.scale")]
    // pub mod robonomics {}
    //
    // let create_call = robonomics::tx().cps().create_node(
    //     parent.map(NodeId::from),
    //     meta_data,
    //     payload_data,
    // );
    //
    // let result = client.api
    //     .tx()
    //     .sign_and_submit_then_watch_default(&create_call, keypair)
    //     .await?
    //     .wait_for_finalized_success()
    //     .await?;
    //
    // // Extract NodeCreated event to get the new node ID
    // let node_id = extract_node_id_from_events(&result.events)?;

    display::tree::error(&format!(
        "Extrinsic submission not implemented yet. This requires:\n\
         1. A running Robonomics node with CPS pallet\n\
         2. Generated subxt metadata\n\
         \n\
         To generate metadata, run:\n\
         {}\n\
         {}",
        "subxt metadata --url ws://localhost:9944 > metadata.scale".bright_cyan(),
        "subxt codegen --file metadata.scale > src/robonomics_runtime.rs".bright_cyan()
    ));

    // Example of successful output:
    println!("\n{}", "Example output (with live node):".bright_yellow());
    display::tree::success(&format!("Node created with ID: {}", 42.to_string().bright_cyan()));

    Ok(())
}
