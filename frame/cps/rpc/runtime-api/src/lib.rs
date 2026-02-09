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
pub use pallet_robonomics_cps::offchain::{MetaRecord, NodeOperation, PayloadRecord};
pub use pallet_robonomics_cps::NodeId;

sp_api::decl_runtime_apis! {
    /// Runtime API for querying indexed CPS data
    pub trait CpsIndexerApi {
        /// Get meta records within optional time range
        ///
        /// # Arguments
        /// * `node_id` - Optional node_id filter
        /// * `from` - Start timestamp (inclusive), None for all
        /// * `to` - End timestamp (inclusive), None for all
        ///
        /// # Returns
        /// Vector of MetaRecord structures
        fn get_meta_records(node_id: Option<NodeId>, from: Option<u64>, to: Option<u64>) -> Vec<MetaRecord>;

        /// Get payload records within optional time range
        ///
        /// # Arguments
        /// * `node_id` - Optional node_id filter
        /// * `from` - Start timestamp (inclusive), None for all
        /// * `to` - End timestamp (inclusive), None for all
        ///
        /// # Returns
        /// Vector of PayloadRecord structures
        fn get_payload_records(node_id: Option<NodeId>, from: Option<u64>, to: Option<u64>) -> Vec<PayloadRecord>;

        /// Get node operations within optional time range
        ///
        /// # Arguments
        /// * `node_id` - Optional node_id filter
        /// * `from` - Start timestamp (inclusive), None for all
        /// * `to` - End timestamp (inclusive), None for all
        ///
        /// # Returns
        /// Vector of NodeOperation structures
        fn get_node_operations(node_id: Option<NodeId>, from: Option<u64>, to: Option<u64>) -> Vec<NodeOperation>;
    }
}
