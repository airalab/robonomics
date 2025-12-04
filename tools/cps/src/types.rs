// CPS Pallet types for subxt integration
//
// These types mirror the pallet-robonomics-cps types but are designed
// for use with subxt. They need to match the on-chain types exactly.

use serde::{Deserialize, Serialize};
use subxt::ext::codec::{Decode, Encode};

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

/// Encrypted data variants matching the pallet's DefaultEncryptedData
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub enum EncryptedData {
    XChaCha20Poly1305(Vec<u8>),
    AesGcm256(Vec<u8>),
    ChaCha20Poly1305(Vec<u8>),
}

/// Node data can be either plain or encrypted
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub enum NodeData {
    Plain(Vec<u8>),
    Encrypted(EncryptedData),
}

impl NodeData {
    pub fn plain(data: impl Into<Vec<u8>>) -> Self {
        Self::Plain(data.into())
    }

    pub fn encrypted_xchacha(data: impl Into<Vec<u8>>) -> Self {
        Self::Encrypted(EncryptedData::XChaCha20Poly1305(data.into()))
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Plain(data) => data,
            Self::Encrypted(EncryptedData::XChaCha20Poly1305(data)) => data,
            Self::Encrypted(EncryptedData::AesGcm256(data)) => data,
            Self::Encrypted(EncryptedData::ChaCha20Poly1305(data)) => data,
        }
    }

    pub fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted(_))
    }
}

/// Node structure as stored on-chain (simplified for query purposes)
#[derive(Debug, Clone, Encode, Decode)]
pub struct Node {
    pub owner: [u8; 32],
    pub parent: Option<NodeId>,
    pub meta: Option<NodeData>,
    pub payload: Option<NodeData>,
    pub path: Vec<NodeId>,
}
