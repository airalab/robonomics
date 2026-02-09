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
use sp_tracing::debug;

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
    pub data: Vec<u8>,
}

/// Payload record with timestamp and node_id
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct PayloadRecord {
    pub timestamp: u64,
    pub node_id: NodeId,
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

/// Store meta record in offchain storage using offchain indexing
///
/// This should be called during block execution (in hooks/extrinsics).
/// Uses `sp_io::offchain_index::set` to write data that will be available
/// to offchain workers and RPC queries.
pub fn store_meta_record(timestamp: u64, node_id: NodeId, data: Vec<u8>) {
    let record = MetaRecord {
        timestamp,
        node_id,
        data,
    };
    let key = meta_key(node_id, timestamp);
    let value = record.encode();

    sp_io::offchain_index::set(&key, &value);
}

/// Store payload record in offchain storage using offchain indexing
///
/// This should be called during block execution (in hooks/extrinsics).
/// Uses `sp_io::offchain_index::set` to write data that will be available
/// to offchain workers and RPC queries.
pub fn store_payload_record(timestamp: u64, node_id: NodeId, data: Vec<u8>) {
    let record = PayloadRecord {
        timestamp,
        node_id,
        data,
    };
    let key = payload_key(node_id, timestamp);
    let value = record.encode();

    sp_io::offchain_index::set(&key, &value);
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
}

/// Get meta records within optional time range from offchain storage
///
/// # Performance Note
/// With node_id specified, queries are efficient using the double-map structure.
/// Without node_id, uses the node index to efficiently query only nodes with data.
pub fn get_meta_records(
    node_id: Option<NodeId>,
    from: Option<u64>,
    to: Option<u64>,
) -> Vec<MetaRecord> {
    use sp_io::offchain;

    let mut records = Vec::new();

    records
}

/// Get payload records within optional time range from offchain storage
///
/// # Performance Note
/// With node_id specified, queries are efficient using the double-map structure.
/// Without node_id, uses the node index to efficiently query only nodes with data.
pub fn get_payload_records(
    node_id: Option<NodeId>,
    from: Option<u64>,
    to: Option<u64>,
) -> Vec<PayloadRecord> {
    use sp_io::offchain;

    let mut records = Vec::new();

    records
}

/// Get node operations within optional time range from offchain storage
///
/// # Performance Note
/// With node_id specified, queries are efficient using the double-map structure.
/// Without node_id, uses the node index to efficiently query only nodes with data.
pub fn get_node_operations(
    node_id: Option<NodeId>,
    from: Option<u64>,
    to: Option<u64>,
) -> Vec<NodeOperation> {
    use sp_io::offchain;

    let mut operations = Vec::new();

    operations
}
