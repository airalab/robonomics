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
//! Offchain storage helpers for CPS indexer
//!
//! This module provides helper functions for storing and retrieving
//! indexed CPS data in offchain storage.

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use crate::NodeId;

/// Metadata record with timestamp and node_id
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct MetaRecord {
    pub timestamp: u64,
    pub node_id: NodeId,
    #[cfg_attr(feature = "std", serde(with = "hex_serde"))]
    pub data: Vec<u8>,
}

/// Payload record with timestamp and node_id
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct PayloadRecord {
    pub timestamp: u64,
    pub node_id: NodeId,
    #[cfg_attr(feature = "std", serde(with = "hex_serde"))]
    pub data: Vec<u8>,
}

/// Node operation type
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum OperationType {
    /// Node created with optional parent
    Create(Option<NodeId>),
    /// Node moved from old parent to new parent
    Move(Option<NodeId>, NodeId),
    /// Node deleted
    Delete,
}

/// Node operation record with timestamp and node_id
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct NodeOperation {
    pub timestamp: u64,
    pub node_id: NodeId,
    pub operation: OperationType,
}

#[cfg(feature = "std")]
mod hex_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("0x{}", hex::encode(data)))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s.strip_prefix("0x").unwrap_or(&s);
        hex::decode(s).map_err(serde::de::Error::custom)
    }
}

/// Storage key prefix for meta records
pub const META_PREFIX: &[u8] = b"cps::meta::";

/// Storage key prefix for payload records
pub const PAYLOAD_PREFIX: &[u8] = b"cps::payload::";

/// Storage key prefix for node operations
pub const OPERATIONS_PREFIX: &[u8] = b"cps::operations::";

/// Generate storage key for meta record (node_id first for efficient lookups)
pub fn meta_key(node_id: NodeId, timestamp: u64) -> Vec<u8> {
    let mut key = META_PREFIX.to_vec();
    key.extend_from_slice(&node_id.0.to_le_bytes());
    key.extend_from_slice(&timestamp.to_le_bytes());
    key
}

/// Generate storage key for payload record (node_id first for efficient lookups)
pub fn payload_key(node_id: NodeId, timestamp: u64) -> Vec<u8> {
    let mut key = PAYLOAD_PREFIX.to_vec();
    key.extend_from_slice(&node_id.0.to_le_bytes());
    key.extend_from_slice(&timestamp.to_le_bytes());
    key
}

/// Generate storage key for node operation (node_id first for efficient lookups)
pub fn operation_key(node_id: NodeId, timestamp: u64) -> Vec<u8> {
    let mut key = OPERATIONS_PREFIX.to_vec();
    key.extend_from_slice(&node_id.0.to_le_bytes());
    key.extend_from_slice(&timestamp.to_le_bytes());
    key
}

/// Store meta record in offchain storage
#[cfg(feature = "std")]
pub fn store_meta_record(timestamp: u64, node_id: NodeId, data: Vec<u8>) {
    use sp_io::offchain;
    
    let record = MetaRecord { timestamp, node_id, data };
    let key = meta_key(node_id, timestamp);
    let value = record.encode();
    
    offchain::local_storage_set(
        sp_core::offchain::StorageKind::PERSISTENT,
        &key,
        &value,
    );
}

/// Store payload record in offchain storage
#[cfg(feature = "std")]
pub fn store_payload_record(timestamp: u64, node_id: NodeId, data: Vec<u8>) {
    use sp_io::offchain;
    
    let record = PayloadRecord { timestamp, node_id, data };
    let key = payload_key(node_id, timestamp);
    let value = record.encode();
    
    offchain::local_storage_set(
        sp_core::offchain::StorageKind::PERSISTENT,
        &key,
        &value,
    );
}

/// Store node operation in offchain storage
#[cfg(feature = "std")]
pub fn store_node_operation(timestamp: u64, node_id: NodeId, operation: OperationType) {
    use sp_io::offchain;
    
    let operation = NodeOperation {
        timestamp,
        node_id,
        operation,
    };
    let key = operation_key(node_id, timestamp);
    let value = operation.encode();
    
    offchain::local_storage_set(
        sp_core::offchain::StorageKind::PERSISTENT,
        &key,
        &value,
    );
}

/// Get meta records within optional time range from offchain storage
///
/// # Performance Note
/// With node_id specified, queries are efficient using the double-map structure.
/// Without node_id, scans a configurable range. In production, maintain an index
/// of active node_ids or use bounded queries.
pub fn get_meta_records(from: Option<u64>, to: Option<u64>, node_id: Option<NodeId>) -> Vec<MetaRecord> {
    use sp_io::offchain;
    
    let mut records = Vec::new();
    
    if let Some(nid) = node_id {
        // Efficient: query specific node's records
        let from = from.unwrap_or(0);
        let to = to.unwrap_or(u64::MAX);
        
        for timestamp in from..=to.min(from + 10000) { // Limit iteration
            let key = meta_key(nid, timestamp);
            
            if let Some(value) = offchain::local_storage_get(
                sp_core::offchain::StorageKind::PERSISTENT,
                &key,
            ) {
                if let Ok(record) = MetaRecord::decode(&mut &value[..]) {
                    records.push(record);
                }
            }
        }
    } else {
        // Less efficient: scan multiple nodes
        let from = from.unwrap_or(0);
        let to = to.unwrap_or(from + 1000); // Smaller default range
        
        for nid in 0..1000 {
            for timestamp in from..=to.min(from + 1000) {
                let key = meta_key(NodeId(nid), timestamp);
                
                if let Some(value) = offchain::local_storage_get(
                    sp_core::offchain::StorageKind::PERSISTENT,
                    &key,
                ) {
                    if let Ok(record) = MetaRecord::decode(&mut &value[..]) {
                        records.push(record);
                    }
                }
            }
        }
    }
    
    records
}

/// Get payload records within optional time range from offchain storage
///
/// # Performance Note
/// With node_id specified, queries are efficient using the double-map structure.
/// Without node_id, scans a configurable range.
pub fn get_payload_records(from: Option<u64>, to: Option<u64>, node_id: Option<NodeId>) -> Vec<PayloadRecord> {
    use sp_io::offchain;
    
    let mut records = Vec::new();
    
    if let Some(nid) = node_id {
        let from = from.unwrap_or(0);
        let to = to.unwrap_or(u64::MAX);
        
        for timestamp in from..=to.min(from + 10000) {
            let key = payload_key(nid, timestamp);
            
            if let Some(value) = offchain::local_storage_get(
                sp_core::offchain::StorageKind::PERSISTENT,
                &key,
            ) {
                if let Ok(record) = PayloadRecord::decode(&mut &value[..]) {
                    records.push(record);
                }
            }
        }
    } else {
        let from = from.unwrap_or(0);
        let to = to.unwrap_or(from + 1000);
        
        for nid in 0..1000 {
            for timestamp in from..=to.min(from + 1000) {
                let key = payload_key(NodeId(nid), timestamp);
                
                if let Some(value) = offchain::local_storage_get(
                    sp_core::offchain::StorageKind::PERSISTENT,
                    &key,
                ) {
                    if let Ok(record) = PayloadRecord::decode(&mut &value[..]) {
                        records.push(record);
                    }
                }
            }
        }
    }
    
    records
}

/// Get node operations within optional time range from offchain storage
///
/// # Performance Note
/// With node_id specified, queries are efficient using the double-map structure.
/// Without node_id, scans a configurable range.
pub fn get_node_operations(from: Option<u64>, to: Option<u64>, node_id: Option<NodeId>) -> Vec<NodeOperation> {
    use sp_io::offchain;
    
    let mut operations = Vec::new();
    
    if let Some(nid) = node_id {
        let from = from.unwrap_or(0);
        let to = to.unwrap_or(u64::MAX);
        
        for timestamp in from..=to.min(from + 10000) {
            let key = operation_key(nid, timestamp);
            
            if let Some(value) = offchain::local_storage_get(
                sp_core::offchain::StorageKind::PERSISTENT,
                &key,
            ) {
                if let Ok(operation) = NodeOperation::decode(&mut &value[..]) {
                    operations.push(operation);
                }
            }
        }
    } else {
        let from = from.unwrap_or(0);
        let to = to.unwrap_or(from + 1000);
        
        for nid in 0..1000 {
            for timestamp in from..=to.min(from + 1000) {
                let key = operation_key(NodeId(nid), timestamp);
                
                if let Some(value) = offchain::local_storage_get(
                    sp_core::offchain::StorageKind::PERSISTENT,
                    &key,
                ) {
                    if let Ok(operation) = NodeOperation::decode(&mut &value[..]) {
                        operations.push(operation);
                    }
                }
            }
        }
    }
    
    operations
}
