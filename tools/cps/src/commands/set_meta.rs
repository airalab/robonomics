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

use libcps::blockchain::{Client, Config};
use libcps::crypto::EncryptionAlgorithm;
use crate::display;
use anyhow::Result;
use colored::*;
use std::str::FromStr;

pub async fn execute(config: &Config, node_id: u64, data: String, encrypt: bool, cipher: &str) -> Result<()> {
    display::tree::progress("Connecting to blockchain...");
    
    let client = Client::new(config).await?;
    let keypair = client.require_keypair()?;

    display::tree::info(&format!("Connected to {}", config.ws_url));
    display::tree::info(&format!("Updating metadata for node {node_id}"));

    // Parse cipher algorithm
    let algorithm = EncryptionAlgorithm::from_str(cipher)
        .map_err(|e| anyhow::anyhow!("Invalid cipher: {}", e))?;

    if encrypt {
        display::tree::info(&format!("🔐 Using encryption algorithm: {}", algorithm));
        display::tree::warning("Encryption not yet fully implemented (requires recipient public key)");
    }

    // In a real implementation:
    // let set_meta_call = robonomics::tx().cps().set_meta(
    //     NodeId(node_id),
    //     Some(NodeData::plain(data.as_bytes())),
    // );
    // 
    // client.api
    //     .tx()
    //     .sign_and_submit_then_watch_default(&set_meta_call, keypair)
    //     .await?
    //     .wait_for_finalized_success()
    //     .await?;

    display::tree::error(&format!(
        "Extrinsic submission not implemented yet. Requires running node and metadata.\n\
         See {} command for details.",
        "create".bright_cyan()
    ));

    println!("\n{}", "Example output (with live node):".bright_yellow());
    display::tree::success(&format!("Metadata updated for node {}", node_id.to_string().bright_cyan()));

    Ok(())
}
