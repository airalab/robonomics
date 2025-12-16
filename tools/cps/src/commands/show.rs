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
//! Show command implementation.

use crate::display;
use anyhow::Result;
use libcps::blockchain::{Client, Config};
use libcps::node::Node;

pub async fn execute(config: &Config, node_id: u64, _decrypt: bool) -> Result<()> {
    display::tree::progress("Connecting to blockchain...");

    let client = Client::new(config).await?;

    display::tree::info(&format!("Connected to {}", config.ws_url));
    display::tree::progress(&format!("Fetching node {node_id}..."));

    // Query node using Node API
    let node = Node::new(&client, node_id);
    let node_info = node.query().await?;

    // Display node information
    let meta_str = String::from_utf8(node_info.meta.as_bytes().to_vec()).ok();
    let payload_str = String::from_utf8(node_info.payload.as_bytes().to_vec()).ok();

    display::tree::print_tree(
        node_id,
        &hex::encode(&node_info.owner),
        meta_str.as_deref(),
        payload_str.as_deref(),
        &node_info.children,
    );

    Ok(())
}
