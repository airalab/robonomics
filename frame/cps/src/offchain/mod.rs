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
//! Offchain worker implementation for CPS indexer
//!
//! This module implements an offchain worker that monitors on-chain CPS events
//! and indexes them for historical queries via the RPC API.

pub mod storage;

use crate::{Config, NodeId};
use frame_system::pallet_prelude::*;
use sp_std::vec::Vec;
use storage::OperationType;

/// Index CPS data from the current block
///
/// This function is called by the offchain worker hook and processes
/// events from the current block to extract and index CPS-related data.
pub fn index_cps_data<T: Config>(block_number: BlockNumberFor<T>) {
    log::debug!(
        target: "cps-indexer",
        "Indexing CPS data for block {:?}",
        block_number
    );
    
    // Get current Unix timestamp in milliseconds from system
    let timestamp = sp_io::offchain::timestamp().unix_millis();
    
    // Process events from the current block
    process_cps_events::<T>(timestamp);
    
    log::debug!(
        target: "cps-indexer",
        "Completed indexing for block {:?}",
        block_number
    );
}

/// Process CPS events and store them in offchain storage
///
/// Note: Event processing requires runtime-specific integration.
/// Runtime implementers should call the index_* functions directly
/// from their event handlers or provide a custom implementation that
/// properly converts RuntimeEvent to CPS Event.
fn process_cps_events<T: Config>(timestamp: u64) {
    log::trace!(
        target: "cps-indexer",
        "Ready to process events for timestamp {}",
        timestamp
    );
    
    // TODO: Runtime integration
    // The runtime should implement event processing by:
    // 1. Iterating through frame_system::Pallet::<T>::events()
    // 2. Matching on RuntimeEvent to extract CPS events
    // 3. Calling index_meta_record, index_payload_record, or index_node_operation
    //
    // Example (to be implemented in runtime):
    // for event_record in frame_system::Pallet::<T>::events() {
    //     match event_record.event {
    //         RuntimeEvent::Cps(cps_event) => match cps_event {
    //             Event::NodeCreated(node_id, parent_id, _) => {
    //                 index_node_operation(timestamp, node_id, OperationType::Create(parent_id));
    //             }
    //             Event::MetaSet(node_id, _) => {
    //                 if let Some(node) = Pallet::<T>::nodes(node_id) {
    //                     if let Some(meta) = node.meta {
    //                         index_meta_record(timestamp, node_id, meta.encode());
    //                     }
    //                 }
    //             }
    //             // ... other events
    //         },
    //         _ => {}
    //     }
    // }
}

/// Index a metadata record
///
/// This should be called by runtime event handlers when metadata is set.
pub fn index_meta_record(timestamp: u64, node_id: NodeId, data: Vec<u8>) {
    #[cfg(feature = "std")]
    storage::store_meta_record(timestamp, node_id, data);
    
    log::trace!(
        target: "cps-indexer",
        "Indexed meta record for node {:?} at timestamp {}",
        node_id,
        timestamp
    );
}

/// Index a payload record
///
/// This should be called by runtime event handlers when payload is set.
pub fn index_payload_record(timestamp: u64, node_id: NodeId, data: Vec<u8>) {
    #[cfg(feature = "std")]
    storage::store_payload_record(timestamp, node_id, data);
    
    log::trace!(
        target: "cps-indexer",
        "Indexed payload record for node {:?} at timestamp {}",
        node_id,
        timestamp
    );
}

/// Index a node operation
///
/// This should be called by runtime event handlers for node lifecycle events.
pub fn index_node_operation(timestamp: u64, node_id: NodeId, operation: OperationType) {
    #[cfg(feature = "std")]
    storage::store_node_operation(timestamp, node_id, operation.clone());
    
    log::trace!(
        target: "cps-indexer",
        "Indexed node operation '{:?}' for node {:?} at timestamp {}",
        operation,
        node_id,
        timestamp
    );
}
