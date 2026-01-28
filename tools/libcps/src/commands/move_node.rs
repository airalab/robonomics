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
//! Move node command implementation.

use crate::display;
use anyhow::Result;
use colored::*;
use libcps::blockchain::{Client, Config};
use libcps::node::Node;

pub async fn execute(config: &Config, node_id: u64, new_parent_id: u64) -> Result<()> {
    display::tree::progress("Connecting to blockchain...");

    let client = Client::new(config).await?;
    let _keypair = client.require_keypair()?;

    display::tree::info(&format!("Connected to {}", config.ws_url));
    display::tree::info(&format!("Moving node {node_id} to parent {new_parent_id}"));

    // Move node using Node API
    let node = Node::new(&client, node_id);

    let spinner = display::banner::spinner("Submitting transaction...");
    node.move_to(new_parent_id).await?;
    spinner.finish_and_clear();

    display::tree::success(&format!(
        "Node {} moved to parent {}",
        node_id.to_string().bright_cyan(),
        new_parent_id.to_string().bright_cyan()
    ));

    Ok(())
}
