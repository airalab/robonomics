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
use libcps::crypto::{Cipher, EncryptedMessage};
use libcps::node::{EncryptedData, Node, NodeData};
use parity_scale_codec::Decode;
use std::future::Future;
use std::pin::Pin;

pub async fn execute(config: &Config, cipher: Option<&Cipher>, node_id: u64) -> Result<()> {
    display::progress("Connecting to blockchain...");

    let client = Client::new(config).await?;

    display::info(&format!("Connected to {}", config.ws_url));
    display::progress(&format!("Fetching node tree from node {node_id}..."));

    // Print the tree recursively
    print_node_tree(&client, node_id, cipher, "", true).await?;

    Ok(())
}

/// Recursively print a node and all its children in tree format
fn print_node_tree<'a>(
    client: &'a Client,
    node_id: u64,
    cipher: Option<&'a Cipher>,
    prefix: &'a str,
    is_last: bool,
) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
    Box::pin(async move {
        // Query node using Node API
        let node = Node::new(client, node_id);
        let node_info = node.query().await?;

        let node_data_to_string = |nd| match nd {
            NodeData::Plain(bytes) => {
                String::from_utf8(bytes.0).map_err(|_| anyhow::anyhow!("Unvalid UTF-8 character"))
            }
            NodeData::Encrypted(EncryptedData::Aead(bytes)) => {
                let message: EncryptedMessage = Decode::decode(&mut &bytes.0[..])
                    .map_err(|e| anyhow::anyhow!("Failed to decode encrypted metadata: {}", e))?;
                if let Some(cipher) = cipher {
                    let decrypted = cipher
                        .decrypt(&message, None)
                        .map_err(|e| anyhow::anyhow!("Failed to decrypt message: {}.", e))?;
                    String::from_utf8(decrypted)
                        .map_err(|_| anyhow::anyhow!("Unvalid UTF-8 character"))
                } else {
                    serde_json::to_string(&message).map_err(|e| {
                        anyhow::anyhow!("Failed to convert encrypted message into JSON: {}.", e)
                    })
                }
            }
        };

        // Try to decrypt if requested and data is encrypted
        let meta_str = match node_info.meta {
            Some(meta) => Some(node_data_to_string(meta)?),
            _ => None,
        };

        let payload_str = match node_info.payload {
            Some(payload) => Some(node_data_to_string(payload)?),
            _ => None,
        };

        // Print this node
        display::tree::print_node_recursive(
            node_id,
            node_info.owner,
            meta_str.as_deref(),
            payload_str.as_deref(),
            prefix,
            is_last,
        );

        // Recursively print children
        if !node_info.children.is_empty() {
            let child_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}|   ", prefix)
            };

            for (i, child_id) in node_info.children.iter().enumerate() {
                let is_last_child = i == node_info.children.len() - 1;
                print_node_tree(client, *child_id, cipher, &child_prefix, is_last_child).await?;
            }
        }

        Ok(())
    })
}
