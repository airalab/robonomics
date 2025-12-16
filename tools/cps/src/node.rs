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
//! use libcps::{Client, Config, node::{Node, CreateNodeParams}};
//! use libcps::crypto::EncryptionAlgorithm;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config {
//!         ws_url: "ws://localhost:9944".to_string(),
//!         suri: Some("//Alice".to_string()),
//!     };
//!     let client = Client::new(&config).await?;
//!     
//!     // Create a new node
//!     let params = CreateNodeParams {
//!         parent: None,
//!         meta: Some(b"sensor".to_vec()),
//!         payload: Some(b"22.5C".to_vec()),
//!         encrypt: false,
//!         algorithm: EncryptionAlgorithm::XChaCha20Poly1305,
//!         keypair_type: libcps::crypto::KeypairType::Sr25519,
//!         recipient_public: None,
//!     };
//!     
//!     let result = Node::create(&client, params).await?;
//!     let node = Node::new(&client, result.node_id);
//!     
//!     // Query the node
//!     let info = node.query().await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Blocking Usage (Sync Context)
//!
//! ```no_run
//! use libcps::{Client, Config, node::{Node, UpdateNodeParams}};
//! use libcps::crypto::EncryptionAlgorithm;
//!
//! fn main() -> anyhow::Result<()> {
//!     let rt = tokio::runtime::Runtime::new()?;
//!     let _guard = rt.enter();
//!     
//!     let config = Config {
//!         ws_url: "ws://localhost:9944".to_string(),
//!         suri: Some("//Alice".to_string()),
//!     };
//!     let client = rt.block_on(Client::new(&config))?;
//!     
//!     let node = Node::new(&client, 5);
//!     
//!     let params = UpdateNodeParams {
//!         node_id: 5,
//!         data: b"updated".to_vec(),
//!         encrypt: false,
//!         algorithm: EncryptionAlgorithm::XChaCha20Poly1305,
//!         keypair_type: libcps::crypto::KeypairType::Sr25519,
//!         recipient_public: None,
//!     };
//!     
//!     // Use blocking method - no .await needed
//!     node.set_payload_blocking(params)?;
//!     
//!     Ok(())
//! }
//! ```

use crate::blockchain::Client;
use crate::crypto::{EncryptionAlgorithm, KeypairType};
use crate::types::NodeData;
use anyhow::{anyhow, Result};
use sp_core::Pair;

/// Parameters for creating a new CPS node.
#[derive(Debug, Clone)]
pub struct CreateNodeParams {
    /// Optional parent node ID (None for root nodes)
    pub parent: Option<u64>,
    /// Optional metadata for the node
    pub meta: Option<Vec<u8>>,
    /// Optional payload for the node
    pub payload: Option<Vec<u8>>,
    /// Whether to encrypt data
    pub encrypt: bool,
    /// Encryption algorithm to use
    pub algorithm: EncryptionAlgorithm,
    /// Keypair type for encryption
    pub keypair_type: KeypairType,
    /// Optional recipient public key for encryption (required if encrypt=true)
    pub recipient_public: Option<Vec<u8>>,
}

/// Result of creating a node operation.
#[derive(Debug, Clone)]
pub struct CreateNodeResult {
    /// The ID of the newly created node
    pub node_id: u64,
    /// Whether the operation was successful
    pub success: bool,
    /// Optional message describing the result
    pub message: Option<String>,
    /// Optional extrinsic hash of the transaction
    pub extrinsic_hash: Option<String>,
}

impl CreateNodeResult {
    /// Get the extrinsic hash of the transaction.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use libcps::node::CreateNodeResult;
    /// # let result = CreateNodeResult {
    /// #     node_id: 0,
    /// #     success: true,
    /// #     message: None,
    /// #     extrinsic_hash: Some("0x1234".to_string()),
    /// # };
    /// if let Some(hash) = result.extrinsic_hash() {
    ///     println!("Tx: {}", hash);
    /// }
    /// ```
    pub fn extrinsic_hash(&self) -> Option<&str> {
        self.extrinsic_hash.as_deref()
    }
}

/// Parameters for updating node metadata or payload.
#[derive(Debug, Clone)]
pub struct UpdateNodeParams {
    /// The data to set
    pub data: Vec<u8>,
    /// Whether to encrypt the data
    pub encrypt: bool,
    /// Encryption algorithm to use
    pub algorithm: EncryptionAlgorithm,
    /// Keypair type for encryption  
    pub keypair_type: KeypairType,
    /// Optional recipient public key for encryption
    pub recipient_public: Option<Vec<u8>>,
}

/// Result of an update operation.
#[derive(Debug, Clone)]
pub struct UpdateNodeResult {
    /// Whether the operation was successful
    pub success: bool,
    /// Optional message describing the result
    pub message: Option<String>,
    /// Optional extrinsic hash of the transaction
    pub extrinsic_hash: Option<String>,
}

impl UpdateNodeResult {
    /// Get the extrinsic hash of the transaction.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use libcps::node::UpdateNodeResult;
    /// # let result = UpdateNodeResult {
    /// #     success: true,
    /// #     message: None,
    /// #     extrinsic_hash: Some("0x5678".to_string()),
    /// # };
    /// if let Some(hash) = result.extrinsic_hash() {
    ///     println!("Tx: {}", hash);
    /// }
    /// ```
    pub fn extrinsic_hash(&self) -> Option<&str> {
        self.extrinsic_hash.as_deref()
    }
}

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
    /// * `id` - The node ID
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
    pub fn new(client: &'a Client, id: u64) -> Self {
        Self { client, id }
    }

    /// Get the node ID.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Create a new node on the blockchain.
    ///
    /// This is a static method that creates a new node and returns both the
    /// result and a `Node` handle to the newly created node.
    ///
    /// # Arguments
    ///
    /// * `client` - Reference to the blockchain client
    /// * `params` - Parameters for node creation
    ///
    /// # Returns
    ///
    /// A `CreateNodeResult` with details about the created node.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libcps::{Client, Config, node::{Node, CreateNodeParams}};
    /// use libcps::crypto::EncryptionAlgorithm;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let config = Config {
    ///     ws_url: "ws://localhost:9944".to_string(),
    ///     suri: Some("//Alice".to_string()),
    /// };
    /// let client = Client::new(&config).await?;
    ///
    /// let params = CreateNodeParams {
    ///     parent: None,
    ///     meta: Some(b"metadata".to_vec()),
    ///     payload: Some(b"data".to_vec()),
    ///     encrypt: false,
    ///     algorithm: EncryptionAlgorithm::XChaCha20Poly1305,
    ///     keypair_type: libcps::crypto::KeypairType::Sr25519,
    ///     recipient_public: None,
    /// };
    ///
    /// let result = Node::create(&client, params).await?;
    /// println!("Created node: {}", result.node_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(_client: &Client, _params: CreateNodeParams) -> Result<CreateNodeResult> {
        // TODO: Implement actual node creation once metadata is available
        Err(anyhow!(
            "Node creation not yet implemented. Requires running node with CPS pallet and generated metadata."
        ))
    }

    /// Create a new node on the blockchain (blocking).
    ///
    /// This is the blocking variant of `create()`. It uses the current tokio
    /// runtime handle to block on the async operation.
    ///
    /// # Arguments
    ///
    /// * `client` - Reference to the blockchain client
    /// * `params` - Parameters for node creation
    ///
    /// # Returns
    ///
    /// A `CreateNodeResult` with details about the created node.
    pub fn create_blocking(client: &Client, params: CreateNodeParams) -> Result<CreateNodeResult> {
        tokio::runtime::Handle::current().block_on(Self::create(client, params))
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
    /// # Arguments
    ///
    /// * `params` - Parameters for the update
    ///
    /// # Returns
    ///
    /// An `UpdateNodeResult` with operation status.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libcps::{Client, Config, node::{Node, UpdateNodeParams}};
    /// use libcps::crypto::EncryptionAlgorithm;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// # let config = Config {
    /// #     ws_url: "ws://localhost:9944".to_string(),
    /// #     suri: Some("//Alice".to_string()),
    /// # };
    /// # let client = Client::new(&config).await?;
    /// let node = Node::new(&client, 5);
    ///
    /// let params = UpdateNodeParams {
    ///     node_id: 5,
    ///     data: b"new metadata".to_vec(),
    ///     encrypt: false,
    ///     algorithm: EncryptionAlgorithm::XChaCha20Poly1305,
    ///     keypair_type: libcps::crypto::KeypairType::Sr25519,
    ///     recipient_public: None,
    /// };
    ///
    /// node.set_meta(params).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_meta(&self, _params: UpdateNodeParams) -> Result<UpdateNodeResult> {
        // TODO: Implement actual metadata update
        Err(anyhow!(
            "Metadata update not yet implemented. Requires running node with CPS pallet and generated metadata."
        ))
    }

    /// Update the metadata of this node (blocking).
    ///
    /// This is the blocking variant of `set_meta()`.
    pub fn set_meta_blocking(&self, params: UpdateNodeParams) -> Result<UpdateNodeResult> {
        tokio::runtime::Handle::current().block_on(self.set_meta(params))
    }

    /// Update the payload of this node.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for the update
    ///
    /// # Returns
    ///
    /// An `UpdateNodeResult` with operation status.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use libcps::{Client, Config, node::{Node, UpdateNodeParams}};
    /// use libcps::crypto::EncryptionAlgorithm;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// # let config = Config {
    /// #     ws_url: "ws://localhost:9944".to_string(),
    /// #     suri: Some("//Alice".to_string()),
    /// # };
    /// # let client = Client::new(&config).await?;
    /// let node = Node::new(&client, 5);
    ///
    /// let params = UpdateNodeParams {
    ///     node_id: 5,
    ///     data: b"23.1C".to_vec(),
    ///     encrypt: false,
    ///     algorithm: EncryptionAlgorithm::XChaCha20Poly1305,
    ///     keypair_type: libcps::crypto::KeypairType::Sr25519,
    ///     recipient_public: None,
    /// };
    ///
    /// node.set_payload(params).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_payload(&self, _params: UpdateNodeParams) -> Result<UpdateNodeResult> {
        // TODO: Implement actual payload update
        Err(anyhow!(
            "Payload update not yet implemented. Requires running node with CPS pallet and generated metadata."
        ))
    }

    /// Update the payload of this node (blocking).
    ///
    /// This is the blocking variant of `set_payload()`.
    pub fn set_payload_blocking(&self, params: UpdateNodeParams) -> Result<UpdateNodeResult> {
        tokio::runtime::Handle::current().block_on(self.set_payload(params))
    }

    /// Move this node to a new parent in the tree.
    ///
    /// # Arguments
    ///
    /// * `new_parent_id` - The ID of the new parent node
    ///
    /// # Returns
    ///
    /// Success or error.
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
    /// node.move_to(3).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn move_to(&self, _new_parent_id: u64) -> Result<()> {
        // TODO: Implement actual node move
        Err(anyhow!(
            "Node move not yet implemented. Requires running node with CPS pallet and generated metadata."
        ))
    }

    /// Move this node to a new parent in the tree (blocking).
    ///
    /// This is the blocking variant of `move_to()`.
    pub fn move_to_blocking(&self, new_parent_id: u64) -> Result<()> {
        tokio::runtime::Handle::current().block_on(self.move_to(new_parent_id))
    }

    /// Delete this node from the tree.
    ///
    /// This method consumes the Node handle, preventing further use after deletion.
    /// The node must have no children to be deleted.
    ///
    /// # Returns
    ///
    /// Success or error.
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
    /// node.delete().await?;
    /// // node is no longer accessible here
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(self) -> Result<()> {
        // TODO: Implement actual node deletion
        Err(anyhow!(
            "Node deletion not yet implemented. Requires running node with CPS pallet and generated metadata."
        ))
    }

    /// Delete this node from the tree (blocking).
    ///
    /// This is the blocking variant of `delete()`. It consumes the Node handle.
    pub fn delete_blocking(self) -> Result<()> {
        tokio::runtime::Handle::current().block_on(self.delete())
    }
}

/// Prepare NodeData with optional encryption.
///
/// This is a utility function that handles data preparation for node operations.
///
/// # Arguments
///
/// * `data` - Raw data bytes
/// * `should_encrypt` - Whether to encrypt the data
/// * `sender` - Sender keypair for encryption
/// * `recipient_public` - Recipient public key (required if should_encrypt=true)
/// * `algorithm` - Encryption algorithm to use
///
/// # Returns
///
/// Prepared `NodeData` ready for blockchain submission.
pub fn prepare_node_data<P>(
    data: &[u8],
    should_encrypt: bool,
    sender: &P,
    recipient_public: Option<&P::Public>,
    algorithm: EncryptionAlgorithm,
) -> Result<NodeData>
where
    P: Pair + crate::crypto::DeriveSharedSecret,
    P::Public: AsRef<[u8]> + sp_core::crypto::UncheckedFrom<[u8; 32]>,
{
    if should_encrypt {
        let recipient = recipient_public
            .ok_or_else(|| anyhow!("Recipient public key required for encryption"))?;

        let encrypted_json_bytes = crate::crypto::encrypt(data, sender, recipient, algorithm)?;

        // The encrypt function returns JSON as Vec<u8>
        // Store it in the appropriate EncryptedData variant based on algorithm
        Ok(match algorithm {
            EncryptionAlgorithm::XChaCha20Poly1305 => NodeData::Encrypted(
                crate::types::EncryptedData::XChaCha20Poly1305(encrypted_json_bytes),
            ),
            EncryptionAlgorithm::AesGcm256 => {
                NodeData::Encrypted(crate::types::EncryptedData::AesGcm256(encrypted_json_bytes))
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => NodeData::Encrypted(
                crate::types::EncryptedData::ChaCha20Poly1305(encrypted_json_bytes),
            ),
        })
    } else {
        Ok(NodeData::plain(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_plain_node_data() {
        let data = b"test data";
        let node_data = NodeData::plain(data);

        // Verify it's plain data
        assert!(matches!(node_data, NodeData::Plain(_)));
    }

    #[test]
    fn test_create_node_params() {
        let params = CreateNodeParams {
            parent: Some(0),
            meta: Some(b"metadata".to_vec()),
            payload: Some(b"payload".to_vec()),
            encrypt: false,
            algorithm: EncryptionAlgorithm::XChaCha20Poly1305,
            keypair_type: KeypairType::Sr25519,
            recipient_public: None,
        };

        assert_eq!(params.parent, Some(0));
        assert_eq!(params.encrypt, false);
    }

    #[test]
    fn test_node_new() {
        // This is a minimal test since we can't create a real Client without a running node
        // In a real test environment with a test node, you would do:
        // let client = Client::new(&test_config).await?;
        // let node = Node::new(&client, 42);
        // assert_eq!(node.id(), 42);
    }

    #[test]
    fn test_create_node_result_extrinsic_hash() {
        let result = CreateNodeResult {
            node_id: 42,
            success: true,
            message: Some("Node created".to_string()),
            extrinsic_hash: Some("0x1234567890abcdef".to_string()),
        };

        assert_eq!(result.extrinsic_hash(), Some("0x1234567890abcdef"));
    }

    #[test]
    fn test_create_node_result_no_extrinsic_hash() {
        let result = CreateNodeResult {
            node_id: 42,
            success: true,
            message: None,
            extrinsic_hash: None,
        };

        assert_eq!(result.extrinsic_hash(), None);
    }

    #[test]
    fn test_update_node_result_extrinsic_hash() {
        let result = UpdateNodeResult {
            success: true,
            message: Some("Node updated".to_string()),
            extrinsic_hash: Some("0xfedcba0987654321".to_string()),
        };

        assert_eq!(result.extrinsic_hash(), Some("0xfedcba0987654321"));
    }

    #[test]
    fn test_update_node_result_no_extrinsic_hash() {
        let result = UpdateNodeResult {
            success: false,
            message: Some("Update failed".to_string()),
            extrinsic_hash: None,
        };

        assert_eq!(result.extrinsic_hash(), None);
    }

    #[test]
    fn test_update_node_params() {
        let params = UpdateNodeParams {
            data: b"updated data".to_vec(),
            encrypt: true,
            algorithm: EncryptionAlgorithm::AesGcm256,
            keypair_type: KeypairType::Ed25519,
            recipient_public: Some(vec![1, 2, 3]),
        };

        assert_eq!(params.data, b"updated data");
        assert_eq!(params.encrypt, true);
        assert_eq!(params.algorithm, EncryptionAlgorithm::AesGcm256);
    }
}
