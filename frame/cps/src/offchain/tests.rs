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
//! Tests for CPS offchain indexer

use super::storage::*;
use crate::NodeId;
use parity_scale_codec::{Decode, Encode};

#[test]
fn test_storage_key_generation() {
    let node_id = NodeId(42);
    let timestamp = 1704067200u64;
    
    // Test meta key generation
    let meta_key = meta_key(node_id, timestamp);
    assert!(meta_key.starts_with(META_PREFIX));
    assert_eq!(meta_key.len(), META_PREFIX.len() + 8 + 8); // prefix + node_id + timestamp
    
    // Test payload key generation
    let payload_key = payload_key(node_id, timestamp);
    assert!(payload_key.starts_with(PAYLOAD_PREFIX));
    assert_eq!(payload_key.len(), PAYLOAD_PREFIX.len() + 8 + 8);
    
    // Test operation key generation
    let operation_key = operation_key(node_id, timestamp);
    assert!(operation_key.starts_with(OPERATIONS_PREFIX));
    assert_eq!(operation_key.len(), OPERATIONS_PREFIX.len() + 8 + 8);
}

#[test]
fn test_storage_key_uniqueness() {
    let node_id1 = NodeId(1);
    let node_id2 = NodeId(2);
    let timestamp1 = 1000u64;
    let timestamp2 = 2000u64;
    
    // Different node_ids should produce different keys
    let key1 = meta_key(node_id1, timestamp1);
    let key2 = meta_key(node_id2, timestamp1);
    assert_ne!(key1, key2);
    
    // Different timestamps should produce different keys
    let key3 = meta_key(node_id1, timestamp1);
    let key4 = meta_key(node_id1, timestamp2);
    assert_ne!(key3, key4);
}

#[test]
fn test_meta_record_encoding() {
    let node_id = NodeId(123);
    let timestamp = 1704067200u64;
    let data = vec![1, 2, 3, 4, 5];
    
    let record = MetaRecord {
        timestamp,
        node_id,
        data: data.clone(),
    };
    
    // Test encode/decode round trip
    let encoded = record.encode();
    let decoded = MetaRecord::decode(&mut &encoded[..]).unwrap();
    
    assert_eq!(decoded.timestamp, timestamp);
    assert_eq!(decoded.node_id, node_id);
    assert_eq!(decoded.data, data);
}

#[test]
fn test_payload_record_encoding() {
    let node_id = NodeId(456);
    let timestamp = 1704070800u64;
    let data = vec![10, 20, 30];
    
    let record = PayloadRecord {
        timestamp,
        node_id,
        data: data.clone(),
    };
    
    let encoded = record.encode();
    let decoded = PayloadRecord::decode(&mut &encoded[..]).unwrap();
    
    assert_eq!(decoded.timestamp, timestamp);
    assert_eq!(decoded.node_id, node_id);
    assert_eq!(decoded.data, data);
}

#[test]
fn test_node_operation_encoding() {
    let node_id = NodeId(789);
    let timestamp = 1704074400u64;
    
    // Test Create operation
    let create_op = NodeOperation {
        timestamp,
        node_id,
        operation: OperationType::Create(Some(NodeId(100))),
    };
    
    let encoded = create_op.encode();
    let decoded = NodeOperation::decode(&mut &encoded[..]).unwrap();
    
    assert_eq!(decoded.timestamp, timestamp);
    assert_eq!(decoded.node_id, node_id);
    assert_eq!(decoded.operation, OperationType::Create(Some(NodeId(100))));
    
    // Test Move operation
    let move_op = NodeOperation {
        timestamp,
        node_id,
        operation: OperationType::Move(Some(NodeId(1)), NodeId(2)),
    };
    
    let encoded = move_op.encode();
    let decoded = NodeOperation::decode(&mut &encoded[..]).unwrap();
    assert_eq!(decoded.operation, OperationType::Move(Some(NodeId(1)), NodeId(2)));
    
    // Test Delete operation
    let delete_op = NodeOperation {
        timestamp,
        node_id,
        operation: OperationType::Delete,
    };
    
    let encoded = delete_op.encode();
    let decoded = NodeOperation::decode(&mut &encoded[..]).unwrap();
    assert_eq!(decoded.operation, OperationType::Delete);
}

#[test]
fn test_operation_type_enum() {
    // Test all operation types
    let create = OperationType::Create(None);
    let create_with_parent = OperationType::Create(Some(NodeId(10)));
    let move_op = OperationType::Move(Some(NodeId(1)), NodeId(2));
    let delete = OperationType::Delete;
    
    // Ensure they encode/decode correctly
    assert_eq!(OperationType::decode(&mut &create.encode()[..]).unwrap(), create);
    assert_eq!(OperationType::decode(&mut &create_with_parent.encode()[..]).unwrap(), create_with_parent);
    assert_eq!(OperationType::decode(&mut &move_op.encode()[..]).unwrap(), move_op);
    assert_eq!(OperationType::decode(&mut &delete.encode()[..]).unwrap(), delete);
}

#[cfg(feature = "std")]
#[test]
fn test_serde_serialization() {
    use serde_json;
    
    let node_id = NodeId(42);
    let timestamp = 1704067200u64;
    let data = vec![72, 101, 108, 108, 111]; // "Hello"
    
    let record = MetaRecord {
        timestamp,
        node_id,
        data,
    };
    
    // Test JSON serialization
    let json = serde_json::to_string(&record).unwrap();
    assert!(json.contains("timestamp"));
    assert!(json.contains("nodeId"));
    assert!(json.contains("0x48656c6c6f")); // "Hello" in hex
    
    // Test JSON deserialization
    let deserialized: MetaRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.timestamp, record.timestamp);
    assert_eq!(deserialized.node_id, record.node_id);
    assert_eq!(deserialized.data, record.data);
}
