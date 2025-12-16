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
//! // Create plain data using From trait
//! let plain: NodeData = "sensor reading: 23.5C".into();
//! // Or explicitly
//! let plain = NodeData::from("sensor reading: 23.5C");
//! // Or using plain() method
//! let plain = NodeData::plain("sensor reading: 23.5C");
//!
//! // Create encrypted data
//! let encrypted = NodeData::encrypted(EncryptedData::XChaCha20Poly1305(vec![1, 2, 3, 4]));
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
/// use libcps::types::{NodeData, EncryptedData};
///
/// // Create plain data using From trait
/// let plain: NodeData = "temperature: 22.5C".into();
/// let plain = NodeData::from(vec![1, 2, 3]);
///
/// // Or using plain() method
/// let plain = NodeData::plain("temperature: 22.5C");
///
/// // Encrypted data
/// let encrypted = NodeData::encrypted(EncryptedData::XChaCha20Poly1305(vec![1, 2, 3, 4]));
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

    /// Create encrypted data with specified encryption type.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::types::{NodeData, EncryptedData};
    ///
    /// let encrypted = NodeData::encrypted(EncryptedData::XChaCha20Poly1305(vec![1, 2, 3, 4]));
    /// let encrypted = NodeData::encrypted(EncryptedData::AesGcm256(vec![1, 2, 3]));
    /// ```
    pub fn encrypted(data: EncryptedData) -> Self {
        Self::Encrypted(data)
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
    /// use libcps::types::{NodeData, EncryptedData};
    ///
    /// let plain = NodeData::plain("hello");
    /// let encrypted = NodeData::encrypted(EncryptedData::XChaCha20Poly1305(vec![1, 2, 3]));
    ///
    /// assert!(!plain.is_encrypted());
    /// assert!(encrypted.is_encrypted());
    /// ```
    pub fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted(_))
    }
}

// From trait implementations for ergonomic NodeData creation
impl From<Vec<u8>> for NodeData {
    fn from(data: Vec<u8>) -> Self {
        Self::Plain(data)
    }
}

impl From<&[u8]> for NodeData {
    fn from(data: &[u8]) -> Self {
        Self::Plain(data.to_vec())
    }
}

impl From<String> for NodeData {
    fn from(data: String) -> Self {
        Self::Plain(data.into_bytes())
    }
}

impl From<&str> for NodeData {
    fn from(data: &str) -> Self {
        Self::Plain(data.as_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // NodeId Tests
    // ========================================================================

    #[test]
    fn test_node_id_creation() {
        let id = NodeId(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_node_id_from_u64() {
        let id = NodeId::from(100u64);
        assert_eq!(id.0, 100);
    }

    #[test]
    fn test_node_id_into_u64() {
        let id = NodeId(50);
        let val: u64 = id.into();
        assert_eq!(val, 50);
    }

    #[test]
    fn test_node_id_equality() {
        let id1 = NodeId(10);
        let id2 = NodeId(10);
        let id3 = NodeId(20);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    // ========================================================================
    // NodeData::from() Tests
    // ========================================================================

    #[test]
    fn test_nodedata_from_string() {
        let data = NodeData::from("hello world");
        assert!(!data.is_encrypted());
        assert_eq!(data.as_bytes(), b"hello world");
    }

    #[test]
    fn test_nodedata_from_vec_u8() {
        let vec = vec![1, 2, 3, 4, 5];
        let data = NodeData::from(vec.clone());
        assert!(!data.is_encrypted());
        assert_eq!(data.as_bytes(), &vec[..]);
    }

    #[test]
    fn test_nodedata_from_bytes_slice() {
        let bytes: &[u8] = b"test data";
        let data = NodeData::from(bytes);
        assert!(!data.is_encrypted());
        assert_eq!(data.as_bytes(), bytes);
    }

    #[test]
    fn test_nodedata_from_empty_string() {
        let data = NodeData::from("");
        assert!(!data.is_encrypted());
        assert_eq!(data.as_bytes(), b"");
    }

    #[test]
    fn test_nodedata_from_empty_vec() {
        let data = NodeData::from(Vec::<u8>::new());
        assert!(!data.is_encrypted());
        assert_eq!(data.as_bytes(), b"");
    }

    // ========================================================================
    // NodeData::plain() Tests
    // ========================================================================

    #[test]
    fn test_nodedata_plain_string() {
        let data = NodeData::plain("sensor reading");
        assert!(!data.is_encrypted());
        assert_eq!(data.as_bytes(), b"sensor reading");
    }

    #[test]
    fn test_nodedata_plain_bytes() {
        let bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let data = NodeData::plain(bytes.clone());
        assert!(!data.is_encrypted());
        assert_eq!(data.as_bytes(), &bytes[..]);
    }

    #[test]
    fn test_nodedata_plain_unicode() {
        let unicode = "Hello 世界 🌍";
        let data = NodeData::plain(unicode);
        assert!(!data.is_encrypted());
        assert_eq!(data.as_bytes(), unicode.as_bytes());
    }

    // ========================================================================
    // NodeData Encrypted Variants Tests
    // ========================================================================

    #[test]
    fn test_nodedata_encrypted_xchacha() {
        let encrypted_bytes = vec![1, 2, 3, 4];
        let data = NodeData::encrypted(EncryptedData::XChaCha20Poly1305(encrypted_bytes.clone()));
        assert!(data.is_encrypted());
        assert_eq!(data.as_bytes(), &encrypted_bytes[..]);
    }

    #[test]
    fn test_encrypted_data_xchacha20() {
        let bytes = vec![1, 2, 3];
        let encrypted = EncryptedData::XChaCha20Poly1305(bytes.clone());
        assert_eq!(encrypted, EncryptedData::XChaCha20Poly1305(bytes));
    }

    #[test]
    fn test_encrypted_data_aesgcm256() {
        let bytes = vec![4, 5, 6];
        let encrypted = EncryptedData::AesGcm256(bytes.clone());
        assert_eq!(encrypted, EncryptedData::AesGcm256(bytes));
    }

    #[test]
    fn test_encrypted_data_chacha20() {
        let bytes = vec![7, 8, 9];
        let encrypted = EncryptedData::ChaCha20Poly1305(bytes.clone());
        assert_eq!(encrypted, EncryptedData::ChaCha20Poly1305(bytes));
    }

    // ========================================================================
    // NodeData::is_encrypted() Tests
    // ========================================================================

    #[test]
    fn test_plain_is_not_encrypted() {
        let data = NodeData::plain("test");
        assert!(!data.is_encrypted());
    }

    #[test]
    fn test_encrypted_xchacha_is_encrypted() {
        let data = NodeData::encrypted(EncryptedData::XChaCha20Poly1305(vec![1, 2, 3]));
        assert!(data.is_encrypted());
    }

    #[test]
    fn test_encrypted_aesgcm_is_encrypted() {
        let data = NodeData::Encrypted(EncryptedData::AesGcm256(vec![1, 2, 3]));
        assert!(data.is_encrypted());
    }

    #[test]
    fn test_encrypted_chacha_is_encrypted() {
        let data = NodeData::Encrypted(EncryptedData::ChaCha20Poly1305(vec![1, 2, 3]));
        assert!(data.is_encrypted());
    }

    // ========================================================================
    // NodeData::as_bytes() Tests
    // ========================================================================

    #[test]
    fn test_as_bytes_plain() {
        let data = NodeData::plain("hello");
        assert_eq!(data.as_bytes(), b"hello");
    }

    #[test]
    fn test_as_bytes_encrypted_xchacha() {
        let bytes = vec![1, 2, 3, 4, 5];
        let data = NodeData::encrypted(EncryptedData::XChaCha20Poly1305(bytes.clone()));
        assert_eq!(data.as_bytes(), &bytes[..]);
    }

    #[test]
    fn test_as_bytes_encrypted_aesgcm() {
        let bytes = vec![6, 7, 8];
        let data = NodeData::Encrypted(EncryptedData::AesGcm256(bytes.clone()));
        assert_eq!(data.as_bytes(), &bytes[..]);
    }

    #[test]
    fn test_as_bytes_encrypted_chacha() {
        let bytes = vec![9, 10, 11];
        let data = NodeData::Encrypted(EncryptedData::ChaCha20Poly1305(bytes.clone()));
        assert_eq!(data.as_bytes(), &bytes[..]);
    }

    // ========================================================================
    // NodeData Edge Cases Tests
    // ========================================================================

    #[test]
    fn test_large_data() {
        let large_data = vec![0u8; 1_000_000]; // 1MB of data
        let data = NodeData::plain(large_data.clone());
        assert_eq!(data.as_bytes().len(), 1_000_000);
        assert!(!data.is_encrypted());
    }

    #[test]
    fn test_nodedata_clone() {
        let original = NodeData::plain("test");
        let cloned = original.clone();
        assert_eq!(original, cloned);
        assert_eq!(original.as_bytes(), cloned.as_bytes());
    }

    #[test]
    fn test_nodedata_debug() {
        let data = NodeData::plain("test");
        let debug_str = format!("{:?}", data);
        assert!(debug_str.contains("Plain"));
    }

    #[test]
    fn test_encrypted_data_clone() {
        let original = EncryptedData::XChaCha20Poly1305(vec![1, 2, 3]);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // ========================================================================
    // SCALE Codec Tests (serialization/deserialization)
    // ========================================================================

    #[test]
    fn test_nodedata_encode_decode_plain() {
        use subxt::ext::codec::{Decode, Encode};

        let original = NodeData::plain("test data");
        let encoded = original.encode();
        let decoded = NodeData::decode(&mut &encoded[..]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_nodedata_encode_decode_encrypted() {
        use subxt::ext::codec::{Decode, Encode};

        let original = NodeData::encrypted(EncryptedData::XChaCha20Poly1305(vec![1, 2, 3, 4]));
        let encoded = original.encode();
        let decoded = NodeData::decode(&mut &encoded[..]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_encrypted_data_encode_decode() {
        use subxt::ext::codec::{Decode, Encode};

        let original = EncryptedData::AesGcm256(vec![5, 6, 7]);
        let encoded = original.encode();
        let decoded = EncryptedData::decode(&mut &encoded[..]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_node_id_encode_decode() {
        use subxt::ext::codec::{Decode, Encode};

        let original = NodeId(12345);
        let encoded = original.encode();
        let decoded = NodeId::decode(&mut &encoded[..]).unwrap();
        assert_eq!(original, decoded);
    }

    // ========================================================================
    // JSON Serialization Tests
    // ========================================================================

    #[test]
    fn test_nodedata_json_serialization_plain() {
        let data = NodeData::plain("test");
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_nodedata_json_serialization_encrypted() {
        let data = NodeData::encrypted(EncryptedData::XChaCha20Poly1305(vec![1, 2, 3]));
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_encrypted_data_json_serialization() {
        let data = EncryptedData::ChaCha20Poly1305(vec![4, 5, 6]);
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: EncryptedData = serde_json::from_str(&json).unwrap();
        assert_eq!(data, deserialized);
    }

    // ========================================================================
    // Node Structure Tests
    // ========================================================================

    #[test]
    fn test_node_creation() {
        let node = Node {
            owner: [0u8; 32],
            parent: Some(NodeId(0)),
            meta: Some(NodeData::plain("metadata")),
            payload: Some(NodeData::plain("payload")),
            path: vec![NodeId(0), NodeId(1)],
        };

        assert_eq!(node.parent, Some(NodeId(0)));
        assert_eq!(node.path.len(), 2);
    }

    #[test]
    fn test_node_with_encrypted_data() {
        let node = Node {
            owner: [1u8; 32],
            parent: None,
            meta: Some(NodeData::encrypted(EncryptedData::XChaCha20Poly1305(vec![
                1, 2, 3,
            ]))),
            payload: Some(NodeData::plain("public payload")),
            path: vec![NodeId(0)],
        };

        assert!(node.meta.as_ref().unwrap().is_encrypted());
        assert!(!node.payload.as_ref().unwrap().is_encrypted());
    }

    #[test]
    fn test_node_encode_decode() {
        use subxt::ext::codec::{Decode, Encode};

        let original = Node {
            owner: [42u8; 32],
            parent: Some(NodeId(5)),
            meta: Some(NodeData::plain("meta")),
            payload: Some(NodeData::plain("payload")),
            path: vec![NodeId(1), NodeId(5)],
        };

        let encoded = original.encode();
        let decoded = Node::decode(&mut &encoded[..]).unwrap();

        assert_eq!(original.owner, decoded.owner);
        assert_eq!(original.parent, decoded.parent);
        assert_eq!(original.path, decoded.path);
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
