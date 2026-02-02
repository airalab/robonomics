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

#[cfg(test)]
mod tests;

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
    
    // The offchain worker should read indexed data that was stored during
    // block execution using `sp_io::offchain_index::set()`
    //
    // Event indexing happens in the pallet hooks (on_initialize/on_finalize)
    // where we have access to block timestamp and can store data with proper keys.
    //
    // The offchain worker's role is to:
    // 1. Monitor the indexed data
    // 2. Perform additional off-chain processing if needed
    // 3. The data is already accessible via the storage helpers
    
    log::debug!(
        target: "cps-indexer",
        "Offchain worker ready at block {:?}",
        block_number
    );
}

/// Index a metadata record
///
/// This should be called during block execution (in hooks/extrinsics)
/// where we have access to block timestamp.
pub fn index_meta_record(block_number: u64, node_id: NodeId, data: Vec<u8>) {
    #[cfg(feature = "std")]
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
/// This should be called during block execution (in hooks/extrinsics)
/// where we have access to block timestamp.
pub fn index_payload_record(block_number: u64, node_id: NodeId, data: Vec<u8>) {
    #[cfg(feature = "std")]
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
/// This should be called during block execution (in hooks/extrinsics)
/// where we have access to block timestamp.
pub fn index_node_operation(block_number: u64, node_id: NodeId, operation: OperationType) {
    #[cfg(feature = "std")]
    storage::store_node_operation(block_number, node_id, operation.clone());
    
    log::trace!(
        target: "cps-indexer",
        "Indexed node operation '{:?}' for node {:?} at block {}",
        operation,
        node_id,
        block_number
    );
}
