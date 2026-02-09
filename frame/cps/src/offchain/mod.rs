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

use crate::{Config, Event, NodeId, Nodes};
use frame_system::pallet_prelude::*;
use parity_scale_codec::Encode;
use sp_runtime::SaturatedConversion;
use sp_std::vec::Vec;
use sp_tracing::{debug, trace};

// Re-export storage types.
pub use storage::{MetaRecord, NodeOperation, OperationType, PayloadRecord};

/// Index CPS data from the current block
///
/// This function is called by the offchain worker hook. It reads the event list
/// that was stored during block execution and processes each event to index data
/// into appropriate storage structures.
pub fn index_events<T: Config>(block_number: BlockNumberFor<T>, events: Vec<Event<T>>) {
    let block_num: u64 = block_number.saturated_into();

    debug!(
        target: "cps-indexer",
        "Offchain worker processing block {:?}",
        block_number
    );

    // Process each event
    for event in events {
        match event {
            Event::<T>::NodeCreated(node_id, parent_id, _) => {
                storage::store_node_operation(
                    block_num,
                    node_id,
                    storage::OperationType::Create(parent_id),
                );
                trace!(
                    target: "cps-indexer",
                    "Indexed NodeCreated: node_id={:?}, parent={:?}",
                    node_id,
                    parent_id
                );
            }
            Event::<T>::MetaSet(node_id, _) => {
                if let Some(node) = <Nodes<T>>::get(&node_id) {
                    let data = node.meta.encode();
                    storage::store_meta_record(block_num, node_id, data);
                    trace!(
                        target: "cps-indexer",
                        "Indexed MetaSet: node_id={:?}",
                        node_id
                    );
                }
            }
            Event::<T>::PayloadSet(node_id, _) => {
                if let Some(node) = <Nodes<T>>::get(&node_id) {
                    let data = node.payload.encode();
                    storage::store_payload_record(block_num, node_id, data);
                    trace!(
                        target: "cps-indexer",
                        "Indexed PayloadSet: node_id={:?}",
                        node_id
                    );
                }
            }
            Event::<T>::NodeMoved(node_id, old_parent, new_parent, _) => {
                storage::store_node_operation(
                    block_num,
                    node_id,
                    storage::OperationType::Move(old_parent, new_parent),
                );
                trace!(
                    target: "cps-indexer",
                    "Indexed NodeMoved: node_id={:?}, from={:?}, to={:?}",
                    node_id,
                    old_parent,
                    new_parent
                );
            }
            Event::<T>::NodeDeleted(node_id, _) => {
                storage::store_node_operation(block_num, node_id, storage::OperationType::Delete);
                trace!(
                    target: "cps-indexer",
                    "Indexed NodeDeleted: node_id={:?}",
                    node_id
                );
            }
        }
    }

    debug!(
        target: "cps-indexer",
        "Offchain worker at block {:?} done",
        block_number
    );
}
