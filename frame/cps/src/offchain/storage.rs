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
use sp_std::vec::Vec;

/// Metadata record with timestamp
#[derive(Debug, Clone, Encode, Decode)]
pub struct MetaRecord {
    pub timestamp: u64,
    pub data: Vec<u8>,
}

/// Payload record with timestamp
#[derive(Debug, Clone, Encode, Decode)]
pub struct PayloadRecord {
    pub timestamp: u64,
    pub data: Vec<u8>,
}

/// Node operation record with timestamp
#[derive(Debug, Clone, Encode, Decode)]
pub struct NodeOperation {
    pub timestamp: u64,
    pub operation_type: Vec<u8>,
    pub data: Vec<u8>,
}

/// Storage key prefix for meta records
pub const META_PREFIX: &[u8] = b"cps::meta::";

/// Storage key prefix for payload records
pub const PAYLOAD_PREFIX: &[u8] = b"cps::payload::";

/// Storage key prefix for node operations
pub const OPERATIONS_PREFIX: &[u8] = b"cps::operations::";

/// Generate storage key for meta record
pub fn meta_key(timestamp: u64) -> Vec<u8> {
    let mut key = META_PREFIX.to_vec();
    key.extend_from_slice(&timestamp.to_le_bytes());
    key
}

/// Generate storage key for payload record
pub fn payload_key(timestamp: u64) -> Vec<u8> {
    let mut key = PAYLOAD_PREFIX.to_vec();
    key.extend_from_slice(&timestamp.to_le_bytes());
    key
}

/// Generate storage key for node operation
pub fn operation_key(timestamp: u64) -> Vec<u8> {
    let mut key = OPERATIONS_PREFIX.to_vec();
    key.extend_from_slice(&timestamp.to_le_bytes());
    key
}

/// Store meta record in offchain storage
#[cfg(feature = "std")]
pub fn store_meta_record(timestamp: u64, data: Vec<u8>) {
    use sp_io::offchain;
    
    let record = MetaRecord { timestamp, data };
    let key = meta_key(timestamp);
    let value = record.encode();
    
    offchain::local_storage_set(
        sp_core::offchain::StorageKind::PERSISTENT,
        &key,
        &value,
    );
}

/// Store payload record in offchain storage
#[cfg(feature = "std")]
pub fn store_payload_record(timestamp: u64, data: Vec<u8>) {
    use sp_io::offchain;
    
    let record = PayloadRecord { timestamp, data };
    let key = payload_key(timestamp);
    let value = record.encode();
    
    offchain::local_storage_set(
        sp_core::offchain::StorageKind::PERSISTENT,
        &key,
        &value,
    );
}

/// Store node operation in offchain storage
#[cfg(feature = "std")]
pub fn store_node_operation(timestamp: u64, operation_type: Vec<u8>, data: Vec<u8>) {
    use sp_io::offchain;
    
    let operation = NodeOperation {
        timestamp,
        operation_type,
        data,
    };
    let key = operation_key(timestamp);
    let value = operation.encode();
    
    offchain::local_storage_set(
        sp_core::offchain::StorageKind::PERSISTENT,
        &key,
        &value,
    );
}

/// Get meta records within time range from offchain storage
///
/// # Performance Note
/// This implementation iterates through every timestamp in the range.
/// For production use with large time ranges, consider implementing:
/// - A timestamp index that stores only timestamps with actual records
/// - Compound keys with prefix iteration support
/// - A secondary index for efficient range queries
pub fn get_meta_records(from: u64, to: u64) -> Vec<(u64, Vec<u8>)> {
    use sp_io::offchain;
    
    let mut records = Vec::new();
    
    // Iterate through timestamps in range
    for timestamp in from..=to {
        let key = meta_key(timestamp);
        
        if let Some(value) = offchain::local_storage_get(
            sp_core::offchain::StorageKind::PERSISTENT,
            &key,
        ) {
            if let Ok(record) = MetaRecord::decode(&mut &value[..]) {
                records.push((record.timestamp, record.data));
            }
        }
    }
    
    records
}

/// Get payload records within time range from offchain storage
///
/// # Performance Note
/// This implementation iterates through every timestamp in the range.
/// For production use with large time ranges, consider implementing:
/// - A timestamp index that stores only timestamps with actual records
/// - Compound keys with prefix iteration support
/// - A secondary index for efficient range queries
pub fn get_payload_records(from: u64, to: u64) -> Vec<(u64, Vec<u8>)> {
    use sp_io::offchain;
    
    let mut records = Vec::new();
    
    // Iterate through timestamps in range
    for timestamp in from..=to {
        let key = payload_key(timestamp);
        
        if let Some(value) = offchain::local_storage_get(
            sp_core::offchain::StorageKind::PERSISTENT,
            &key,
        ) {
            if let Ok(record) = PayloadRecord::decode(&mut &value[..]) {
                records.push((record.timestamp, record.data));
            }
        }
    }
    
    records
}

/// Get node operations within time range from offchain storage
///
/// # Performance Note
/// This implementation iterates through every timestamp in the range.
/// For production use with large time ranges, consider implementing:
/// - A timestamp index that stores only timestamps with actual records
/// - Compound keys with prefix iteration support
/// - A secondary index for efficient range queries
pub fn get_node_operations(from: u64, to: u64) -> Vec<(u64, Vec<u8>, Vec<u8>)> {
    use sp_io::offchain;
    
    let mut operations = Vec::new();
    
    // Iterate through timestamps in range
    for timestamp in from..=to {
        let key = operation_key(timestamp);
        
        if let Some(value) = offchain::local_storage_get(
            sp_core::offchain::StorageKind::PERSISTENT,
            &key,
        ) {
            if let Ok(operation) = NodeOperation::decode(&mut &value[..]) {
                operations.push((operation.timestamp, operation.operation_type, operation.data));
            }
        }
    }
    
    operations
}
