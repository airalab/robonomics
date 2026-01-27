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

pub use crate::robonomics_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
pub use crate::robonomics_runtime::api::runtime_types::pallet_robonomics_cps::{
    DefaultEncryptedData as EncryptedData, NodeData, NodeId,
};

/// Helper methods for NodeData type
impl NodeData {
    /// Create an encrypted AEAD NodeData from bytes
    pub fn aead_from(v: Vec<u8>) -> Self {
        NodeData::Encrypted(EncryptedData::Aead(BoundedVec(v)))
    }
}

/// Implement From<Vec<u8>> for NodeData (creates Plain variant)
impl From<Vec<u8>> for NodeData {
    fn from(v: Vec<u8>) -> Self {
        NodeData::Plain(BoundedVec(v))
    }
}

/// Implement From<String> for NodeData (creates Plain variant)
impl From<String> for NodeData {
    fn from(s: String) -> Self {
        Self::from(s.into_bytes())
    }
}

/// Implement From<&str> for NodeData (creates Plain variant)
impl From<&str> for NodeData {
    fn from(s: &str) -> Self {
        Self::from(s.as_bytes().to_vec())
    }
}
