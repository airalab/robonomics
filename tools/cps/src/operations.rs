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
//! Core operations for CPS pallet interactions.
//!
//! This module provides reusable functions for interacting with the CPS pallet
//! that can be used by CLI applications or integrated into other applications.
//! All functions here are UI-agnostic and focus on business logic.

use crate::blockchain::Client;
use crate::types::NodeData;
use crate::crypto::{EncryptionAlgorithm, KeypairType};
use anyhow::{Result, anyhow};
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
}

/// Parameters for updating node metadata or payload.
#[derive(Debug, Clone)]
pub struct UpdateNodeParams {
    /// The node ID to update
    pub node_id: u64,
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
}

/// Query information about a specific node.
///
/// # Arguments
///
/// * `client` - The blockchain client
/// * `node_id` - The ID of the node to query
///
/// # Returns
///
/// Node data if found, or an error if the node doesn't exist or query fails.
///
/// # Example
///
/// ```no_run
/// use libcps::{Client, Config, operations};
///
/// # async fn example() -> anyhow::Result<()> {
/// let config = Config {
///     ws_url: "ws://localhost:9944".to_string(),
///     suri: Some("//Alice".to_string()),
/// };
/// let client = Client::new(&config).await?;
///
/// // Query node 0
/// let node_info = operations::query_node(&client, 0).await?;
/// # Ok(())
/// # }
/// ```
pub async fn query_node(_client: &Client, _node_id: u64) -> Result<()> {
    // TODO: Implement actual blockchain query once metadata is available
    // This would use:
    // let nodes_query = robonomics::storage().cps().nodes(NodeId(node_id));
    // let node = client.api.storage().at_latest().await?
    //     .fetch(&nodes_query).await?
    //     .ok_or_else(|| anyhow!("Node {} not found", node_id))?;
    
    Err(anyhow!(
        "Node query not yet implemented. Requires running node with CPS pallet and generated metadata."
    ))
}

/// Create a new node in the CPS tree.
///
/// This function handles data preparation, optional encryption, and submits
/// the extrinsic to create a node on the blockchain.
///
/// # Arguments
///
/// * `client` - The blockchain client
/// * `params` - Parameters for node creation
///
/// # Returns
///
/// A `CreateNodeResult` with details about the created node.
///
/// # Example
///
/// ```no_run
/// use libcps::{Client, Config, operations::{self, CreateNodeParams}};
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
///     parent: None,  // Root node
///     meta: Some(b"sensor metadata".to_vec()),
///     payload: Some(b"22.5C".to_vec()),
///     encrypt: false,
///     algorithm: EncryptionAlgorithm::XChaCha20Poly1305,
///     keypair_type: libcps::crypto::KeypairType::Sr25519,
///     recipient_public: None,
/// };
///
/// let result = operations::create_node(&client, params).await?;
/// println!("Created node ID: {}", result.node_id);
/// # Ok(())
/// # }
/// ```
pub async fn create_node(_client: &Client, _params: CreateNodeParams) -> Result<CreateNodeResult> {
    // TODO: Implement actual node creation once metadata is available
    // This would:
    // 1. Prepare NodeData (encrypt if requested)
    // 2. Build the create_node extrinsic
    // 3. Sign and submit the extrinsic
    // 4. Wait for finalization
    // 5. Extract node ID from events
    
    Err(anyhow!(
        "Node creation not yet implemented. Requires running node with CPS pallet and generated metadata."
    ))
}

/// Update the metadata of an existing node.
///
/// # Arguments
///
/// * `client` - The blockchain client
/// * `params` - Parameters for the update
///
/// # Returns
///
/// An `UpdateNodeResult` with operation status.
pub async fn set_node_meta(_client: &Client, _params: UpdateNodeParams) -> Result<UpdateNodeResult> {
    // TODO: Implement actual metadata update
    Err(anyhow!(
        "Metadata update not yet implemented. Requires running node with CPS pallet and generated metadata."
    ))
}

/// Update the payload of an existing node.
///
/// # Arguments
///
/// * `client` - The blockchain client
/// * `params` - Parameters for the update
///
/// # Returns
///
/// An `UpdateNodeResult` with operation status.
pub async fn set_node_payload(_client: &Client, _params: UpdateNodeParams) -> Result<UpdateNodeResult> {
    // TODO: Implement actual payload update
    Err(anyhow!(
        "Payload update not yet implemented. Requires running node with CPS pallet and generated metadata."
    ))
}

/// Move a node to a new parent in the tree.
///
/// # Arguments
///
/// * `client` - The blockchain client
/// * `node_id` - The ID of the node to move
/// * `new_parent_id` - The ID of the new parent node
///
/// # Returns
///
/// Success or error.
pub async fn move_node(_client: &Client, _node_id: u64, _new_parent_id: u64) -> Result<()> {
    // TODO: Implement actual node move
    Err(anyhow!(
        "Node move not yet implemented. Requires running node with CPS pallet and generated metadata."
    ))
}

/// Delete a node from the tree.
///
/// The node must have no children to be deleted.
///
/// # Arguments
///
/// * `client` - The blockchain client
/// * `node_id` - The ID of the node to delete
///
/// # Returns
///
/// Success or error.
pub async fn delete_node(_client: &Client, _node_id: u64) -> Result<()> {
    // TODO: Implement actual node deletion
    Err(anyhow!(
        "Node deletion not yet implemented. Requires running node with CPS pallet and generated metadata."
    ))
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
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                NodeData::Encrypted(crate::types::EncryptedData::XChaCha20Poly1305(encrypted_json_bytes))
            }
            EncryptionAlgorithm::AesGcm256 => {
                NodeData::Encrypted(crate::types::EncryptedData::AesGcm256(encrypted_json_bytes))
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                NodeData::Encrypted(crate::types::EncryptedData::ChaCha20Poly1305(encrypted_json_bytes))
            }
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
}
