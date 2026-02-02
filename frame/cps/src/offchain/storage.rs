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

/// CPS event that can be indexed
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum CpsEvent {
    /// Node created [node_id, parent_id]
    NodeCreated(NodeId, Option<NodeId>),
    /// Node metadata set [node_id, data]
    MetaSet(NodeId, Vec<u8>),
    /// Node payload set [node_id, data]
    PayloadSet(NodeId, Vec<u8>),
    /// Node moved [node_id, old_parent, new_parent]
    NodeMoved(NodeId, Option<NodeId>, NodeId),
    /// Node deleted [node_id]
    NodeDeleted(NodeId),
}

/// Event queue for a block
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct EventQueue {
    pub block_number: u64,
    pub events: Vec<CpsEvent>,
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

/// Storage key prefix for node index
const NODE_INDEX_PREFIX: &[u8] = b"cps::node_index::";

/// Storage key prefix for event queue
const EVENT_QUEUE_PREFIX: &[u8] = b"cps::event_queue::";

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

/// Generate storage key for node index entry
/// This tracks which nodes have indexed data for efficient querying
fn node_index_key(node_id: NodeId) -> Vec<u8> {
    let mut key = NODE_INDEX_PREFIX.to_vec();
    key.extend_from_slice(&node_id.0.to_le_bytes());
    key
}

/// Generate storage key for event queue
/// This stores events for a specific block to be processed by offchain worker
fn event_queue_key(block_number: u64) -> Vec<u8> {
    let mut key = EVENT_QUEUE_PREFIX.to_vec();
    key.extend_from_slice(&block_number.to_le_bytes());
    key
}

/// Register a node in the index
/// This allows efficient discovery of which nodes have indexed data
fn register_node_in_index(node_id: NodeId) {
    let key = node_index_key(node_id);
    // Store a simple marker (1 byte) to indicate this node has data
    sp_io::offchain_index::set(&key, &[1u8]);
}

/// Get list of indexed node IDs
/// Returns node IDs that have been registered in the index
fn get_indexed_node_ids() -> Vec<NodeId> {
    use sp_io::offchain;
    
    let mut node_ids = Vec::new();
    
    // Scan for node index entries
    // In production, consider maintaining a more efficient index structure
    for nid in 0..10000 {
        let node_id = NodeId(nid);
        let key = node_index_key(node_id);
        
        if offchain::local_storage_get(
            sp_core::offchain::StorageKind::PERSISTENT,
            &key,
        ).is_some() {
            node_ids.push(node_id);
        }
    }
    
    node_ids
}

/// Store event queue for a block in offchain storage
///
/// This should be called once per block in `on_finalize` hook.
/// Uses `sp_io::offchain_index::set` to write event queue that will be
/// processed by the offchain worker.
pub fn store_event_queue(block_number: u64, events: Vec<CpsEvent>) {
    let queue = EventQueue { block_number, events };
    let key = event_queue_key(block_number);
    let value = queue.encode();
    
    sp_io::offchain_index::set(&key, &value);
    
    log::debug!(
        target: "cps-indexer",
        "Stored event queue for block {} with {} events",
        block_number,
        queue.events.len()
    );
}

/// Retrieve event queue for a block from offchain storage
///
/// This should be called by the offchain worker to process events.
pub fn get_event_queue(block_number: u64) -> Option<EventQueue> {
    use sp_io::offchain;
    
    let key = event_queue_key(block_number);
    
    if let Some(value) = offchain::local_storage_get(
        sp_core::offchain::StorageKind::PERSISTENT,
        &key,
    ) {
        EventQueue::decode(&mut &value[..]).ok()
    } else {
        None
    }
}

/// Store meta record in offchain storage using offchain indexing
///
/// This should be called during block execution (in hooks/extrinsics).
/// Uses `sp_io::offchain_index::set` to write data that will be available
/// to offchain workers and RPC queries.
pub fn store_meta_record(timestamp: u64, node_id: NodeId, data: Vec<u8>) {
    let record = MetaRecord { timestamp, node_id, data };
    let key = meta_key(node_id, timestamp);
    let value = record.encode();
    
    sp_io::offchain_index::set(&key, &value);
    register_node_in_index(node_id);
}

/// Store payload record in offchain storage using offchain indexing
///
/// This should be called during block execution (in hooks/extrinsics).
/// Uses `sp_io::offchain_index::set` to write data that will be available
/// to offchain workers and RPC queries.
pub fn store_payload_record(timestamp: u64, node_id: NodeId, data: Vec<u8>) {
    let record = PayloadRecord { timestamp, node_id, data };
    let key = payload_key(node_id, timestamp);
    let value = record.encode();
    
    sp_io::offchain_index::set(&key, &value);
    register_node_in_index(node_id);
}

/// Store node operation in offchain storage using offchain indexing
///
/// This should be called during block execution (in hooks/extrinsics).
/// Uses `sp_io::offchain_index::set` to write data that will be available
/// to offchain workers and RPC queries.
pub fn store_node_operation(timestamp: u64, node_id: NodeId, operation: OperationType) {
    let operation = NodeOperation {
        timestamp,
        node_id,
        operation,
    };
    let key = operation_key(node_id, timestamp);
    let value = operation.encode();
    
    sp_io::offchain_index::set(&key, &value);
    register_node_in_index(node_id);
}

/// Get meta records within optional time range from offchain storage
///
/// # Performance Note
/// With node_id specified, queries are efficient using the double-map structure.
/// Without node_id, uses the node index to efficiently query only nodes with data.
pub fn get_meta_records(node_id: Option<NodeId>, from: Option<u64>, to: Option<u64>) -> Vec<MetaRecord> {
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
        // Use node index for efficient querying
        let from = from.unwrap_or(0);
        let to = to.unwrap_or(u64::MAX);
        let indexed_nodes = get_indexed_node_ids();
        
        for nid in indexed_nodes {
            for timestamp in from..=to.min(from + 10000) {
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
        }
    }
    
    records
}

/// Get payload records within optional time range from offchain storage
///
/// # Performance Note
/// With node_id specified, queries are efficient using the double-map structure.
/// Without node_id, uses the node index to efficiently query only nodes with data.
pub fn get_payload_records(node_id: Option<NodeId>, from: Option<u64>, to: Option<u64>) -> Vec<PayloadRecord> {
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
        let to = to.unwrap_or(u64::MAX);
        let indexed_nodes = get_indexed_node_ids();
        
        for nid in indexed_nodes {
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
        }
    }
    
    records
}

/// Get node operations within optional time range from offchain storage
///
/// # Performance Note
/// With node_id specified, queries are efficient using the double-map structure.
/// Without node_id, uses the node index to efficiently query only nodes with data.
pub fn get_node_operations(node_id: Option<NodeId>, from: Option<u64>, to: Option<u64>) -> Vec<NodeOperation> {
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
        let to = to.unwrap_or(u64::MAX);
        let indexed_nodes = get_indexed_node_ids();
        
        for nid in indexed_nodes {
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
        }
    }
    
    operations
}
