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
//! This module implements offchain indexing for CPS events using Substrate's
//! `sp_io::offchain_index` API. Data is indexed during block execution and
//! made available for queries via RPC.
//!
//! ## Architecture
//!
//! - **Indexing**: During block execution (in hooks/extrinsics), call `index_*` 
//!   functions which use `sp_io::offchain_index::set()` to write data
//! - **Querying**: RPC queries use `sp_io::offchain::local_storage_get()` to 
//!   read indexed data
//! - **Storage**: Double-map structure with node_id first, then timestamp for
//!   efficient per-node queries
//! - **Node Index**: Maintains a list of node_ids with indexed data for efficient
//!   querying across all nodes

pub mod storage;

#[cfg(test)]
mod tests;

use crate::{Config, NodeId};
use frame_system::pallet_prelude::*;
use sp_runtime::SaturatedConversion;
use sp_std::vec::Vec;
use storage::OperationType;

/// Index CPS data from the current block
///
/// This function is called by the offchain worker hook. It reads the event queue
/// that was stored during block execution and processes each event to index data
/// into appropriate storage structures.
pub fn index_cps_data<T: Config>(block_number: BlockNumberFor<T>) {
    let block_num: u64 = block_number.saturated_into();
    
    log::debug!(
        target: "cps-indexer",
        "Offchain worker processing block {:?}",
        block_number
    );
    
    // Retrieve event queue for this block
    if let Some(queue) = storage::get_event_queue(block_num) {
        log::debug!(
            target: "cps-indexer",
            "Processing {} events from block {}",
            queue.events.len(),
            queue.block_number
        );
        
        // Process each event
        for event in queue.events {
            match event {
                storage::CpsEvent::NodeCreated(node_id, parent_id) => {
                    storage::store_node_operation(
                        block_num,
                        node_id,
                        storage::OperationType::Create(parent_id),
                    );
                    log::trace!(
                        target: "cps-indexer",
                        "Indexed NodeCreated: node_id={:?}, parent={:?}",
                        node_id,
                        parent_id
                    );
                }
                storage::CpsEvent::MetaSet(node_id, data) => {
                    storage::store_meta_record(block_num, node_id, data);
                    log::trace!(
                        target: "cps-indexer",
                        "Indexed MetaSet: node_id={:?}",
                        node_id
                    );
                }
                storage::CpsEvent::PayloadSet(node_id, data) => {
                    storage::store_payload_record(block_num, node_id, data);
                    log::trace!(
                        target: "cps-indexer",
                        "Indexed PayloadSet: node_id={:?}",
                        node_id
                    );
                }
                storage::CpsEvent::NodeMoved(node_id, old_parent, new_parent) => {
                    storage::store_node_operation(
                        block_num,
                        node_id,
                        storage::OperationType::Move(old_parent, new_parent),
                    );
                    log::trace!(
                        target: "cps-indexer",
                        "Indexed NodeMoved: node_id={:?}, from={:?}, to={:?}",
                        node_id,
                        old_parent,
                        new_parent
                    );
                }
                storage::CpsEvent::NodeDeleted(node_id) => {
                    storage::store_node_operation(
                        block_num,
                        node_id,
                        storage::OperationType::Delete,
                    );
                    log::trace!(
                        target: "cps-indexer",
                        "Indexed NodeDeleted: node_id={:?}",
                        node_id
                    );
                }
            }
        }
    }
}

/// Index a metadata record
///
/// This should be called during block execution (in hooks/extrinsics) when
/// node metadata is updated. Uses `sp_io::offchain_index::set()` to write
/// the data for offchain access.
///
/// # Example
/// ```ignore
/// let block_num: u64 = <frame_system::Pallet<T>>::block_number().saturated_into();
/// index_meta_record(block_num, node_id, meta_data);
/// ```
pub fn index_meta_record(block_number: u64, node_id: NodeId, data: Vec<u8>) {
    storage::store_meta_record(block_number, node_id, data);
    
    log::trace!(
        target: "cps-indexer",
        "Indexed meta record for node {:?} at block {}",
        node_id,
        block_number
    );
}

/// Index a payload record
///
/// This should be called during block execution (in hooks/extrinsics) when
/// node payload is updated. Uses `sp_io::offchain_index::set()` to write
/// the data for offchain access.
///
/// # Example
/// ```ignore
/// let block_num: u64 = <frame_system::Pallet<T>>::block_number().saturated_into();
/// index_payload_record(block_num, node_id, payload_data);
/// ```
pub fn index_payload_record(block_number: u64, node_id: NodeId, data: Vec<u8>) {
    storage::store_payload_record(block_number, node_id, data);
    
    log::trace!(
        target: "cps-indexer",
        "Indexed payload record for node {:?} at block {}",
        node_id,
        block_number
    );
}

/// Index a node operation
///
/// This should be called during block execution (in hooks/extrinsics) for
/// node lifecycle events (create/move/delete). Uses `sp_io::offchain_index::set()`
/// to write the data for offchain access.
///
/// # Example
/// ```ignore
/// let block_num: u64 = <frame_system::Pallet<T>>::block_number().saturated_into();
/// index_node_operation(block_num, node_id, OperationType::Create(parent_id));
/// ```
pub fn index_node_operation(block_number: u64, node_id: NodeId, operation: OperationType) {
    storage::store_node_operation(block_number, node_id, operation.clone());
    
    log::trace!(
        target: "cps-indexer",
        "Indexed node operation '{:?}' for node {:?} at block {}",
        operation,
        node_id,
        block_number
    );
}
