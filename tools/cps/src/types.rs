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

use crate::robonomics_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;

pub use crate::robonomics_runtime::api::runtime_types::pallet_robonomics_cps::{
    DefaultEncryptedData as EncryptedData, NodeData, NodeId,
};

use crate::crypto::EncryptionAlgorithm;

/// Helper trait to convert various types into NodeData
impl NodeData {
    /// Create a plain (unencrypted) NodeData from raw bytes
    pub fn plain_from_bytes(data: Vec<u8>) -> Self {
        NodeData::Plain(BoundedVec(data))
    }

    /// Create an encrypted NodeData from encrypted bytes and algorithm info
    pub fn from_encrypted_bytes(encrypted_bytes: Vec<u8>, algorithm: EncryptionAlgorithm) -> Self {
        let bounded_vec = BoundedVec(encrypted_bytes);
        
        let encrypted_data = match algorithm {
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                EncryptedData::XChaCha20Poly1305(bounded_vec)
            }
            EncryptionAlgorithm::AesGcm256 => {
                EncryptedData::AesGcm256(bounded_vec)
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                EncryptedData::ChaCha20Poly1305(bounded_vec)
            }
        };
        
        NodeData::Encrypted(encrypted_data)
    }
}

/// Implement From<String> for NodeData (creates Plain variant)
impl From<String> for NodeData {
    fn from(s: String) -> Self {
        NodeData::plain_from_bytes(s.into_bytes())
    }
}

/// Implement From<&str> for NodeData (creates Plain variant)
impl From<&str> for NodeData {
    fn from(s: &str) -> Self {
        NodeData::plain_from_bytes(s.as_bytes().to_vec())
    }
}

