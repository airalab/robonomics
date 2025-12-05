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
//! Remove node command implementation.

use libcps::blockchain::{Client, Config};
use crate::display;
use anyhow::Result;
use colored::*;
use std::io::{self, Write};

pub async fn execute(config: &Config, node_id: u64, force: bool) -> Result<()> {
    display::tree::progress("Connecting to blockchain...");
    
    let client = Client::new(config).await?;
    let keypair = client.require_keypair()?;

    display::tree::info(&format!("Connected to {}", config.ws_url));

    // Check if node has children
    // let children = fetch_children(node_id)?;
    // if !children.is_empty() {
    //     return Err(anyhow!("Cannot delete node with children. Remove children first."));
    // }

    if !force {
        print!("{} Are you sure you want to delete node {}? (y/N): ", 
            "⚠️".yellow(), 
            node_id.to_string().bright_cyan()
        );
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if !input.trim().eq_ignore_ascii_case("y") {
            display::tree::info("Deletion cancelled");
            return Ok(());
        }
    }

    display::tree::info(&format!("Deleting node {node_id}"));

    // In a real implementation:
    // let delete_call = robonomics::tx().cps().delete_node(NodeId(node_id));
    // 
    // client.api
    //     .tx()
    //     .sign_and_submit_then_watch_default(&delete_call, keypair)
    //     .await?
    //     .wait_for_finalized_success()
    //     .await?;

    display::tree::error(&format!(
        "Extrinsic submission not implemented yet. Requires running node and metadata.\n\
         See {} command for details.",
        "create".bright_cyan()
    ));

    println!("\n{}", "Example output (with live node):".bright_yellow());
    display::tree::success(&format!("Node {} deleted", node_id.to_string().bright_cyan()));

    Ok(())
}
