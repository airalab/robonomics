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
//! Node-oriented API for CPS pallet interactions.
//!
//! This module provides an object-oriented interface for managing CPS nodes
//! on the Robonomics blockchain. It offers both async and blocking methods
//! for all operations, making it suitable for various application contexts.
//!
//! # Architecture
//!
//! The `Node` struct encapsulates a reference to a blockchain client and a
//! node ID, providing methods to interact with that specific node. This
//! design promotes clear ownership semantics and enables natural method
//! chaining patterns.
//!
//! # Async vs Blocking
//!
//! All methods are available in two variants:
//! - **Async methods**: Standard async/await methods for use in tokio contexts
//! - **Blocking methods**: Suffixed with `_blocking`, these use `block_on` internally
//!
//! # Examples
//!
//! ## Async Usage (Recommended)
//!
//! ```no_run
//! use libcps::{Client, Config, node::Node, types::NodeData};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config {
//!         ws_url: "ws://localhost:9944".to_string(),
//!         suri: Some("//Alice".to_string()),
//!     };
//!     let client = Client::new(&config).await?;
//!     
//!     // Create a new node with clean API
//!     let meta: NodeData = "sensor".into();
//!     let payload: NodeData = "22.5C".into();
//!     
//!     let node = Node::create(&client, None, Some(meta), Some(payload)).await?;
//!     
//!     // Query the node
//!     let info = node.query().await?;
//!     
//!     // Update payload - returns ExtrinsicEvents
//!     let new_payload: NodeData = "23.1C".into();
//!     let events = node.set_payload(Some(new_payload)).await?;
//!     println!("Tx: {:?}", events);
//!     
//!     Ok(())
//! }
//! ```

use crate::blockchain::Client;
use crate::types::NodeData;
use anyhow::{anyhow, Result};

/// Placeholder type for ExtrinsicEvents - will be replaced with actual subxt type when available.
///
/// Once the blockchain integration is implemented, this will be:
/// `subxt::blocks::ExtrinsicEvents<PolkadotConfig>`
pub type ExtrinsicEvents = ();  // TODO: Replace with subxt::blocks::ExtrinsicEvents<PolkadotConfig>

/// Information about a CPS node.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    /// Node ID
    pub id: u64,
    /// Node owner account
    pub owner: Vec<u8>,
    /// Optional parent node ID
    pub parent: Option<u64>,
    /// Node metadata
    pub meta: NodeData,
    /// Node payload
    pub payload: NodeData,
    /// Child node IDs
    pub children: Vec<u64>,
}

/// A handle to a specific CPS node on the blockchain.
///
/// The `Node` struct encapsulates the identity of a node and provides
/// methods for all operations on that node. It maintains a reference to
/// the blockchain client for RPC communication.
///
/// # Lifetime
///
/// The `Node` struct holds a reference to a `Client` and therefore has
/// the same lifetime constraints. The client must outlive all `Node`
/// instances created from it.
///
/// # Methods
///
/// All methods are available in both async and blocking variants:
/// - Async: `query()`, `set_meta()`, `set_payload()`, `move_to()`, `delete()`
/// - Blocking: `query_blocking()`, `set_meta_blocking()`, etc.
pub struct Node<'a> {
    #[allow(dead_code)]
    client: &'a Client,
    id: u64,
}

impl<'a> Node<'a> {
    /// Create a new `Node` handle for an existing node.
    ///
    /// This does not create a node on the blockchain; it simply creates a
    /// handle to reference an existing node by its ID.
    ///
    /// # Arguments
    ///
    /// * `client` - Reference to the blockchain client
    /// * `node_id` - The node ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libcps::{Client, Config, node::Node};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let config = Config {
    ///     ws_url: "ws://localhost:9944".to_string(),
    ///     suri: Some("//Alice".to_string()),
    /// };
    /// let client = Client::new(&config).await?;
    ///
    /// // Create a handle to node 5
    /// let node = Node::new(&client, 5);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(client: &'a Client, node_id: u64) -> Self {
        Self { client, id: node_id }
    }

    /// Get the node ID.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Create a new node on the blockchain.
    ///
    /// Factory method that creates a new node and returns a Node instance.
    ///
    /// # Arguments
    ///
    /// * `client` - Reference to the blockchain client
    /// * `parent` - Optional parent node ID (None for root nodes)
    /// * `meta` - Optional metadata for the node
    /// * `payload` - Optional payload for the node
    ///
    /// # Returns
    ///
    /// A `Node` handle to the newly created node.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libcps::{Client, Config, node::Node, types::NodeData};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let config = Config {
    ///     ws_url: "ws://localhost:9944".to_string(),
    ///     suri: Some("//Alice".to_string()),
    /// };
    /// let client = Client::new(&config).await?;
    ///
    /// let meta: NodeData = "metadata".into();
    /// let payload: NodeData = "payload data".into();
    ///
    /// let node = Node::create(&client, None, Some(meta), Some(payload)).await?;
    /// println!("Created node: {}", node.id());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        _client: &'a Client,
        _parent: Option<u64>,
        _meta: Option<NodeData>,
        _payload: Option<NodeData>,
    ) -> Result<Self> {
        // TODO: Implement actual node creation once metadata is available
        Err(anyhow!(
            "Node creation not yet implemented. Requires running node with CPS pallet and generated metadata."
        ))
    }

    /// Query information about this node from the blockchain.
    ///
    /// # Returns
    ///
    /// Node information including metadata, payload, parent, and children.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libcps::{Client, Config, node::Node};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// # let config = Config {
    /// #     ws_url: "ws://localhost:9944".to_string(),
    /// #     suri: Some("//Alice".to_string()),
    /// # };
    /// # let client = Client::new(&config).await?;
    /// let node = Node::new(&client, 5);
    /// let info = node.query().await?;
    /// println!("Node owner: {:?}", info.owner);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query(&self) -> Result<NodeInfo> {
        // TODO: Implement actual blockchain query once metadata is available
        Err(anyhow!(
            "Node query not yet implemented. Requires running node with CPS pallet and generated metadata."
        ))
    }

    /// Query information about this node from the blockchain (blocking).
    ///
    /// This is the blocking variant of `query()`.
    pub fn query_blocking(&self) -> Result<NodeInfo> {
        tokio::runtime::Handle::current().block_on(self.query())
    }

    /// Update the metadata of this node.
    ///
    /// Returns subxt's ExtrinsicEvents containing transaction hash and events.
    ///
    /// # Arguments
    ///
    /// * `meta` - Optional metadata for the node
    ///
    /// # Returns
    ///
    /// ExtrinsicEvents with transaction hash and events.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libcps::{Client, Config, node::Node, types::NodeData};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// # let config = Config {
    /// #     ws_url: "ws://localhost:9944".to_string(),
    /// #     suri: Some("//Alice".to_string()),
    /// # };
    /// # let client = Client::new(&config).await?;
    /// let node = Node::new(&client, 5);
    ///
    /// let meta: NodeData = "new metadata".into();
    /// let events = node.set_meta(Some(meta)).await?;
    /// println!("Tx hash: {:?}", events);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_meta(&self, _meta: Option<NodeData>) -> Result<ExtrinsicEvents> {
        // TODO: Implement actual metadata update
        Err(anyhow!(
            "Metadata update not yet implemented. Requires running node with CPS pallet and generated metadata."
        ))
    }

    /// Update the payload of this node.
    ///
    /// Returns subxt's ExtrinsicEvents containing transaction hash and events.
    ///
    /// # Arguments
    ///
    /// * `payload` - Optional payload for the node
    ///
    /// # Returns
    ///
    /// ExtrinsicEvents with transaction hash and events.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libcps::{Client, Config, node::Node, types::NodeData};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// # let config = Config {
    /// #     ws_url: "ws://localhost:9944".to_string(),
    /// #     suri: Some("//Alice".to_string()),
    /// # };
    /// # let client = Client::new(&config).await?;
    /// let node = Node::new(&client, 5);
    ///
    /// let payload: NodeData = "23.1C".into();
    /// let events = node.set_payload(Some(payload)).await?;
    /// println!("Tx hash: {:?}", events);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_payload(&self, _payload: Option<NodeData>) -> Result<ExtrinsicEvents> {
        // TODO: Implement actual payload update
        Err(anyhow!(
            "Payload update not yet implemented. Requires running node with CPS pallet and generated metadata."
        ))
    }

    /// Move this node to a new parent in the tree.
    ///
    /// Returns subxt's ExtrinsicEvents containing transaction hash and events.
    ///
    /// # Arguments
    ///
    /// * `new_parent` - The ID of the new parent node
    ///
    /// # Returns
    ///
    /// ExtrinsicEvents with transaction hash and events.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libcps::{Client, Config, node::Node};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// # let config = Config {
    /// #     ws_url: "ws://localhost:9944".to_string(),
    /// #     suri: Some("//Alice".to_string()),
    /// # };
    /// # let client = Client::new(&config).await?;
    /// let node = Node::new(&client, 5);
    ///
    /// // Move node 5 to be a child of node 3
    /// let events = node.move_to(3).await?;
    /// println!("Tx hash: {:?}", events);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn move_to(&self, _new_parent: u64) -> Result<ExtrinsicEvents> {
        // TODO: Implement actual node move
        Err(anyhow!(
            "Node move not yet implemented. Requires running node with CPS pallet and generated metadata."
        ))
    }

    /// Delete this node from the tree.
    ///
    /// This method consumes the Node handle, preventing further use after deletion.
    /// The node must have no children to be deleted.
    ///
    /// Returns subxt's ExtrinsicEvents containing transaction hash and events.
    ///
    /// # Returns
    ///
    /// ExtrinsicEvents with transaction hash and events.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libcps::{Client, Config, node::Node};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// # let config = Config {
    /// #     ws_url: "ws://localhost:9944".to_string(),
    /// #     suri: Some("//Alice".to_string()),
    /// # };
    /// # let client = Client::new(&config).await?;
    /// let node = Node::new(&client, 5);
    ///
    /// // Delete the node (consumes self, must have no children)
    /// let events = node.delete().await?;
    /// println!("Tx hash: {:?}", events);
    /// // node is no longer accessible here
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(self) -> Result<ExtrinsicEvents> {
        // TODO: Implement actual node deletion
        Err(anyhow!(
            "Node deletion not yet implemented. Requires running node with CPS pallet and generated metadata."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_new() {
        // This is a minimal test since we can't create a real Client without a running node
        // In a real test environment with a test node, you would do:
        // let client = Client::new(&test_config).await?;
        // let node = Node::new(&client, 42);
        // assert_eq!(node.id(), 42);
    }
}
