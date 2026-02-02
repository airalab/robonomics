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
//! Runtime API for CPS Offchain Indexer
//!
//! This API provides access to historical CPS data collected by the offchain worker.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::vec::Vec;

// Re-export the storage types for use in runtime API implementations
pub use pallet_robonomics_cps::NodeId;

#[cfg(feature = "std")]
pub use pallet_robonomics_cps::offchain::storage::{MetaRecord, PayloadRecord, NodeOperation};

sp_api::decl_runtime_apis! {
    /// Runtime API for querying indexed CPS data
    pub trait CpsIndexerApi {
        /// Get meta records within a time range
        ///
        /// # Arguments
        /// * `from` - Start timestamp (inclusive)
        /// * `to` - End timestamp (inclusive)
        /// * `node_id` - Optional node_id filter
        ///
        /// # Returns
        /// Vector of MetaRecord structures
        fn get_meta_records(from: u64, to: u64, node_id: Option<NodeId>) -> Vec<(u64, NodeId, Vec<u8>)>;
        
        /// Get payload records within a time range
        ///
        /// # Arguments
        /// * `from` - Start timestamp (inclusive)
        /// * `to` - End timestamp (inclusive)
        /// * `node_id` - Optional node_id filter
        ///
        /// # Returns
        /// Vector of PayloadRecord structures
        fn get_payload_records(from: u64, to: u64, node_id: Option<NodeId>) -> Vec<(u64, NodeId, Vec<u8>)>;
        
        /// Get node operations within a time range
        ///
        /// # Arguments
        /// * `from` - Start timestamp (inclusive)
        /// * `to` - End timestamp (inclusive)
        /// * `node_id` - Optional node_id filter
        ///
        /// # Returns
        /// Vector of NodeOperation structures (timestamp, node_id, operation_bytes)
        fn get_node_operations(from: u64, to: u64, node_id: Option<NodeId>) -> Vec<(u64, NodeId, Vec<u8>)>;
    }
}
