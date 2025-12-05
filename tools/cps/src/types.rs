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
//! Type definitions for CPS pallet integration.
//!
//! This module provides type definitions that match the Robonomics CPS pallet
//! for use with subxt blockchain integration.
//!
//! # Overview
//!
//! The types in this module are designed to:
//! - Match on-chain storage types exactly for SCALE encoding/decoding
//! - Provide a convenient API for application developers
//! - Support both plain and encrypted data storage
//!
//! # Examples
//!
//! ```
//! use libcps::types::{NodeId, NodeData, EncryptedData};
//!
//! // Create a node ID
//! let node_id = NodeId(42);
//!
//! // Create plain data
//! let plain = NodeData::plain("sensor reading: 23.5C");
//!
//! // Create encrypted data
//! let encrypted = NodeData::encrypted_xchacha(vec![1, 2, 3, 4]);
//!
//! // Check if data is encrypted
//! assert!(!plain.is_encrypted());
//! assert!(encrypted.is_encrypted());
//! ```

use serde::{Deserialize, Serialize};
use subxt::ext::codec::{Decode, Encode};

/// Node identifier with compact encoding.
///
/// Node IDs are u64 values that use compact SCALE encoding for efficiency.
/// This matches the `NodeId` type in the CPS pallet.
///
/// # Examples
///
/// ```
/// use libcps::types::NodeId;
///
/// let node_id = NodeId(42);
/// let as_u64: u64 = node_id.into();
/// let from_u64 = NodeId::from(100u64);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub struct NodeId(pub u64);

impl From<u64> for NodeId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<NodeId> for u64 {
    fn from(id: NodeId) -> Self {
        id.0
    }
}

/// Encrypted data variants matching the pallet's `DefaultEncryptedData`.
///
/// This enum supports multiple encryption algorithms for different use cases:
///
/// - **XChaCha20Poly1305**: Recommended for most use cases (large nonce space)
/// - **AesGcm256**: Hardware-accelerated on most modern processors
/// - **ChaCha20Poly1305**: Standard ChaCha20-Poly1305 with 96-bit nonce
///
/// # Examples
///
/// ```
/// use libcps::types::EncryptedData;
///
/// let encrypted = EncryptedData::XChaCha20Poly1305(vec![1, 2, 3, 4]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub enum EncryptedData {
    /// XChaCha20-Poly1305 AEAD encryption (24-byte nonce)
    XChaCha20Poly1305(Vec<u8>),
    /// AES-256-GCM AEAD encryption (12-byte nonce)
    AesGcm256(Vec<u8>),
    /// ChaCha20-Poly1305 AEAD encryption (12-byte nonce)
    ChaCha20Poly1305(Vec<u8>),
}

/// Node data container supporting both plain and encrypted storage.
///
/// This enum allows mixed privacy models within the same tree:
/// - Public metadata with encrypted payload
/// - Encrypted metadata with public payload
/// - Both encrypted or both plain
///
/// # Examples
///
/// ```
/// use libcps::types::NodeData;
///
/// // Plain text data
/// let plain = NodeData::plain("temperature: 22.5C");
///
/// // Encrypted data
/// let encrypted = NodeData::encrypted_xchacha(vec![1, 2, 3, 4]);
///
/// // Check type
/// assert!(!plain.is_encrypted());
/// assert!(encrypted.is_encrypted());
///
/// // Access underlying bytes
/// let bytes = plain.as_bytes();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub enum NodeData {
    /// Plain (unencrypted) data
    Plain(Vec<u8>),
    /// Encrypted data with specified algorithm
    Encrypted(EncryptedData),
}

impl NodeData {
    /// Create plain (unencrypted) data.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::types::NodeData;
    ///
    /// let data = NodeData::plain("hello world");
    /// let data_bytes = NodeData::plain(vec![1, 2, 3]);
    /// ```
    pub fn plain(data: impl Into<Vec<u8>>) -> Self {
        Self::Plain(data.into())
    }

    /// Create encrypted data using XChaCha20-Poly1305.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::types::NodeData;
    ///
    /// let encrypted = NodeData::encrypted_xchacha(vec![1, 2, 3, 4]);
    /// ```
    pub fn encrypted_xchacha(data: impl Into<Vec<u8>>) -> Self {
        Self::Encrypted(EncryptedData::XChaCha20Poly1305(data.into()))
    }

    /// Get the underlying bytes regardless of encryption status.
    ///
    /// For encrypted data, this returns the raw encrypted bytes,
    /// not the decrypted plaintext.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::types::NodeData;
    ///
    /// let data = NodeData::plain("hello");
    /// assert_eq!(data.as_bytes(), b"hello");
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Plain(data) => data,
            Self::Encrypted(EncryptedData::XChaCha20Poly1305(data)) => data,
            Self::Encrypted(EncryptedData::AesGcm256(data)) => data,
            Self::Encrypted(EncryptedData::ChaCha20Poly1305(data)) => data,
        }
    }

    /// Check if the data is encrypted.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::types::NodeData;
    ///
    /// let plain = NodeData::plain("hello");
    /// let encrypted = NodeData::encrypted_xchacha(vec![1, 2, 3]);
    ///
    /// assert!(!plain.is_encrypted());
    /// assert!(encrypted.is_encrypted());
    /// ```
    pub fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted(_))
    }
}

/// Node structure as stored on-chain.
///
/// This is a simplified representation for query purposes.
/// The actual on-chain structure may have additional fields.
///
/// # Fields
///
/// * `owner` - Account that owns this node (32-byte account ID)
/// * `parent` - Optional parent node ID
/// * `meta` - Optional metadata (configuration data)
/// * `payload` - Optional payload (operational data)
/// * `path` - Full path from root to this node (for O(1) cycle detection)
///
/// # Examples
///
/// ```
/// use libcps::types::{Node, NodeId, NodeData};
///
/// let node = Node {
///     owner: [0u8; 32],
///     parent: Some(NodeId(0)),
///     meta: Some(NodeData::plain("sensor")),
///     payload: Some(NodeData::plain("22.5C")),
///     path: vec![NodeId(0), NodeId(1)],
/// };
/// ```
#[derive(Debug, Clone, Encode, Decode)]
pub struct Node {
    /// Account that owns this node
    pub owner: [u8; 32],
    /// Optional parent node ID
    pub parent: Option<NodeId>,
    /// Optional metadata (configuration)
    pub meta: Option<NodeData>,
    /// Optional payload (operational data)
    pub payload: Option<NodeData>,
    /// Path from root to this node
    pub path: Vec<NodeId>,
}
