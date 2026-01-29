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
use crate::robonomics_runtime;
use crate::types::{NodeData, NodeId as CpsNodeId};
use anyhow::{anyhow, Result};
use log::{debug, trace};
use sp_core::crypto::AccountId32;
use subxt::PolkadotConfig;

/// Type for extrinsic events from blockchain transactions.
pub type ExtrinsicEvents = subxt::blocks::ExtrinsicEvents<PolkadotConfig>;

/// Information about a CPS node.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    /// Node ID
    pub id: u64,
    /// Node owner account
    pub owner: AccountId32,
    /// Optional parent node ID
    pub parent: Option<u64>,
    /// Node metadata
    pub meta: Option<NodeData>,
    /// Node payload
    pub payload: Option<NodeData>,
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
        Self {
            client,
            id: node_id,
        }
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
        client: &'a Client,
        parent: Option<u64>,
        meta: Option<NodeData>,
        payload: Option<NodeData>,
    ) -> Result<Self> {
        debug!(
            "Creating new CPS node: parent={:?}, has_meta={}, has_payload={}",
            parent,
            meta.is_some(),
            payload.is_some()
        );

        let keypair = client.require_keypair()?;
        trace!("Using keypair for transaction signing");

        // Convert parent to NodeId type
        let parent_id = parent.map(CpsNodeId);

        // Build the create_node transaction
        trace!("Building create_node transaction");
        let create_call = robonomics_runtime::api::tx()
            .cps()
            .create_node(parent_id, meta, payload);

        // Submit and watch the transaction
        trace!("Submitting and watching transaction");
        let events = client
            .api
            .tx()
            .sign_and_submit_then_watch_default(&create_call, keypair)
            .await
            .map_err(|e| anyhow!("Failed to submit create_node transaction: {}", e))?
            .wait_for_finalized_success()
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?;

        // Extract the created node ID from the NodeCreated event
        trace!("Extracting node ID from NodeCreated event");
        let node_created_event = events
            .find_first::<robonomics_runtime::api::cps::events::NodeCreated>()
            .map_err(|e| anyhow!("Failed to find NodeCreated event: {}", e))?
            .ok_or_else(|| anyhow!("NodeCreated event not found in transaction events"))?;

        let node_id = node_created_event.0 .0; // Extract u64 from NodeId(u64)
        debug!("CPS node created successfully: id={}", node_id);

        Ok(Self {
            client,
            id: node_id,
        })
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
        trace!("Querying node {} at latest finalized block", self.id);
        // Get the latest finalized block and query at that block
        let block_hash = self
            .client
            .api
            .backend()
            .latest_finalized_block_ref()
            .await?
            .hash();

        trace!("Latest finalized block hash: {:?}", block_hash);
        self.query_at(block_hash).await
    }

    /// Query information about this node at a specific block hash.
    ///
    /// This is useful when you need to query the node state at a specific point in time.
    ///
    /// # Arguments
    ///
    /// * `block_hash` - The block hash to query at
    ///
    /// # Returns
    ///
    /// Node information including metadata, payload, parent, and children at the specified block.
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
    /// // Get the finalized block hash
    /// let block_hash = client.api.backend().latest_finalized_block_ref().await?.hash();
    /// let info = node.query_at(block_hash).await?;
    /// println!("Node owner: {:?}", info.owner);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_at(&self, block_hash: subxt::utils::H256) -> Result<NodeInfo> {
        // Query the node from storage at specific block
        let node_id = CpsNodeId(self.id);
        let nodes_query = robonomics_runtime::api::storage().cps().nodes(node_id);

        let node = self
            .client
            .api
            .storage()
            .at(block_hash)
            .fetch(&nodes_query)
            .await
            .map_err(|e| anyhow!("Failed to query node storage at block: {}", e))?
            .ok_or_else(|| anyhow!("Node {} not found", self.id))?;

        // Query children
        let children_query = robonomics_runtime::api::storage()
            .cps()
            .nodes_by_parent(node_id);

        let children = self
            .client
            .api
            .storage()
            .at(block_hash)
            .fetch(&children_query)
            .await
            .map_err(|e| anyhow!("Failed to query children at block: {}", e))?
            .map(|v| v.0)
            .unwrap_or(vec![]);

        Ok(NodeInfo {
            id: self.id,
            owner: node.owner.0.into(),
            parent: node.parent.map(|p| p.0),
            meta: node.meta,
            payload: node.payload,
            children: children.iter().map(|id| id.0).collect(),
        })
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
    pub async fn set_meta(&self, meta: Option<NodeData>) -> Result<ExtrinsicEvents> {
        debug!(
            "Setting metadata for node {}: has_data={}",
            self.id,
            meta.is_some()
        );
        let keypair = self.client.require_keypair()?;
        let node_id = CpsNodeId(self.id);

        // Build the set_meta transaction
        trace!("Building set_meta transaction");
        let set_meta_call = robonomics_runtime::api::tx().cps().set_meta(node_id, meta);

        // Submit and watch the transaction
        trace!("Submitting set_meta transaction for node {}", self.id);
        let events = self
            .client
            .api
            .tx()
            .sign_and_submit_then_watch_default(&set_meta_call, keypair)
            .await
            .map_err(|e| anyhow!("Failed to submit set_meta transaction: {}", e))?
            .wait_for_finalized_success()
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?;

        debug!("Metadata updated successfully for node {}", self.id);

        Ok(events)
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
    pub async fn set_payload(&self, payload: Option<NodeData>) -> Result<ExtrinsicEvents> {
        debug!(
            "Setting payload for node {}: has_data={}",
            self.id,
            payload.is_some()
        );
        let keypair = self.client.require_keypair()?;
        let node_id = CpsNodeId(self.id);

        // Build the set_payload transaction
        trace!("Building set_payload transaction");
        let set_payload_call = robonomics_runtime::api::tx()
            .cps()
            .set_payload(node_id, payload);

        // Submit and watch the transaction
        trace!("Submitting set_payload transaction for node {}", self.id);
        let events = self
            .client
            .api
            .tx()
            .sign_and_submit_then_watch_default(&set_payload_call, keypair)
            .await
            .map_err(|e| anyhow!("Failed to submit set_payload transaction: {}", e))?
            .wait_for_finalized_success()
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?;

        debug!("Payload updated successfully for node {}", self.id);
        Ok(events)
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
    pub async fn move_to(&self, new_parent: u64) -> Result<ExtrinsicEvents> {
        let keypair = self.client.require_keypair()?;
        let node_id = CpsNodeId(self.id);
        let new_parent_id = CpsNodeId(new_parent);

        // Build the move_node transaction
        let move_node_call = robonomics_runtime::api::tx()
            .cps()
            .move_node(node_id, new_parent_id);

        // Submit and watch the transaction
        let events = self
            .client
            .api
            .tx()
            .sign_and_submit_then_watch_default(&move_node_call, keypair)
            .await
            .map_err(|e| anyhow!("Failed to submit move_node transaction: {}", e))?
            .wait_for_finalized_success()
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?;

        Ok(events)
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
        let keypair = self.client.require_keypair()?;
        let node_id = CpsNodeId(self.id);

        // Build the delete_node transaction
        let delete_node_call = robonomics_runtime::api::tx().cps().delete_node(node_id);

        // Submit and watch the transaction
        let events = self
            .client
            .api
            .tx()
            .sign_and_submit_then_watch_default(&delete_node_call, keypair)
            .await
            .map_err(|e| anyhow!("Failed to submit delete_node transaction: {}", e))?
            .wait_for_finalized_success()
            .await
            .map_err(|e| anyhow!("Transaction failed: {}", e))?;

        Ok(events)
    }
}
