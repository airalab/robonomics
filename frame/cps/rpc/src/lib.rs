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
//! RPC extension for CPS Offchain Indexer
//!
//! This module provides JSON-RPC endpoints to query historical CPS data
//! collected by the offchain worker.

use jsonrpsee::{
    core::RpcResult,
    proc_macros::rpc,
    types::ErrorObjectOwned,
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

pub use pallet_robonomics_cps_rpc_runtime_api::{CpsIndexerApi as CpsIndexerRuntimeApi, NodeId};

// Re-export the storage structures for JSON serialization
pub use pallet_robonomics_cps::offchain::storage::{MetaRecord, NodeOperation, PayloadRecord};

/// CPS Indexer RPC API
#[rpc(client, server)]
pub trait CpsIndexerRpcApi<BlockHash> {
    /// Get meta records within optional time range
    ///
    /// # Arguments
    /// * `node_id` - Optional node_id filter
    /// * `from` - Start timestamp (inclusive), None for all
    /// * `to` - End timestamp (inclusive), None for all
    /// * `at` - Optional block hash to query at (defaults to best block)
    ///
    /// # Returns
    /// Vector of meta records with timestamps, node_ids and hex-encoded data
    #[method(name = "cps_getMetaRecords")]
    fn get_meta_records(
        &self,
        node_id: Option<u64>,
        from: Option<u64>,
        to: Option<u64>,
        at: Option<BlockHash>,
    ) -> RpcResult<Vec<MetaRecord>>;
    
    /// Get payload records within optional time range
    ///
    /// # Arguments
    /// * `node_id` - Optional node_id filter
    /// * `from` - Start timestamp (inclusive), None for all
    /// * `to` - End timestamp (inclusive), None for all
    /// * `at` - Optional block hash to query at (defaults to best block)
    ///
    /// # Returns
    /// Vector of payload records with timestamps, node_ids and hex-encoded data
    #[method(name = "cps_getPayloadRecords")]
    fn get_payload_records(
        &self,
        node_id: Option<u64>,
        from: Option<u64>,
        to: Option<u64>,
        at: Option<BlockHash>,
    ) -> RpcResult<Vec<PayloadRecord>>;
    
    /// Get node operations within optional time range
    ///
    /// # Arguments
    /// * `node_id` - Optional node_id filter
    /// * `from` - Start timestamp (inclusive), None for all
    /// * `to` - End timestamp (inclusive), None for all
    /// * `at` - Optional block hash to query at (defaults to best block)
    ///
    /// # Returns
    /// Vector of node operations with timestamps, node_ids, and operation types
    #[method(name = "cps_getNodeOperations")]
    fn get_node_operations(
        &self,
        node_id: Option<u64>,
        from: Option<u64>,
        to: Option<u64>,
        at: Option<BlockHash>,
    ) -> RpcResult<Vec<NodeOperation>>;
}

/// Implementation of the CPS Indexer RPC API
pub struct CpsIndexerRpc<C, Block> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<Block>,
}

impl<C, Block> CpsIndexerRpc<C, Block> {
    /// Create new instance of the RPC handler
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block> CpsIndexerRpcApiServer<<Block as BlockT>::Hash> for CpsIndexerRpc<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: CpsIndexerRuntimeApi<Block>,
{
    fn get_meta_records(
        &self,
        node_id: Option<u64>,
        from: Option<u64>,
        to: Option<u64>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<MetaRecord>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        let node_id = node_id.map(NodeId);
        
        api.get_meta_records(at_hash, node_id, from, to)
            .map_err(|e| ErrorObjectOwned::owned(
                1, 
                "Failed to retrieve meta records from runtime", 
                Some(format!("{:?}", e))
            ))
    }
    
    fn get_payload_records(
        &self,
        node_id: Option<u64>,
        from: Option<u64>,
        to: Option<u64>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<PayloadRecord>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        let node_id = node_id.map(NodeId);
        
        api.get_payload_records(at_hash, node_id, from, to)
            .map_err(|e| ErrorObjectOwned::owned(
                1, 
                "Failed to retrieve payload records from runtime", 
                Some(format!("{:?}", e))
            ))
    }
    
    fn get_node_operations(
        &self,
        node_id: Option<u64>,
        from: Option<u64>,
        to: Option<u64>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<NodeOperation>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        let node_id = node_id.map(NodeId);
        
        api.get_node_operations(at_hash, node_id, from, to)
            .map_err(|e| ErrorObjectOwned::owned(
                1, 
                "Failed to retrieve node operations from runtime", 
                Some(format!("{:?}", e))
            ))
    }
}
