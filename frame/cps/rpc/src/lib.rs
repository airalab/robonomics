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
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

pub use pallet_robonomics_cps_rpc_runtime_api::CpsIndexerApi as CpsIndexerRuntimeApi;

/// JSON-serializable meta record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaRecordJson {
    /// Unix timestamp in seconds
    pub timestamp: u64,
    /// Hex-encoded data
    pub data: String,
}

/// JSON-serializable payload record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadRecordJson {
    /// Unix timestamp in seconds
    pub timestamp: u64,
    /// Hex-encoded data
    pub data: String,
}

/// JSON-serializable node operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeOperationJson {
    /// Unix timestamp in seconds
    pub timestamp: u64,
    /// Operation type (e.g., "create", "update", "delete")
    pub operation_type: String,
    /// Hex-encoded operation data
    pub data: String,
}

/// CPS Indexer RPC API
#[rpc(client, server)]
pub trait CpsIndexerRpcApi<BlockHash> {
    /// Get meta records within a time range
    ///
    /// # Arguments
    /// * `from` - Start timestamp (inclusive)
    /// * `to` - End timestamp (inclusive)
    /// * `at` - Optional block hash to query at (defaults to best block)
    ///
    /// # Returns
    /// Vector of meta records with timestamps and hex-encoded data
    #[method(name = "cps_getMetaRecords")]
    fn get_meta_records(
        &self,
        from: u64,
        to: u64,
        at: Option<BlockHash>,
    ) -> RpcResult<Vec<MetaRecordJson>>;
    
    /// Get payload records within a time range
    ///
    /// # Arguments
    /// * `from` - Start timestamp (inclusive)
    /// * `to` - End timestamp (inclusive)
    /// * `at` - Optional block hash to query at (defaults to best block)
    ///
    /// # Returns
    /// Vector of payload records with timestamps and hex-encoded data
    #[method(name = "cps_getPayloadRecords")]
    fn get_payload_records(
        &self,
        from: u64,
        to: u64,
        at: Option<BlockHash>,
    ) -> RpcResult<Vec<PayloadRecordJson>>;
    
    /// Get node operations within a time range
    ///
    /// # Arguments
    /// * `from` - Start timestamp (inclusive)
    /// * `to` - End timestamp (inclusive)
    /// * `at` - Optional block hash to query at (defaults to best block)
    ///
    /// # Returns
    /// Vector of node operations with timestamps, types, and hex-encoded data
    #[method(name = "cps_getNodeOperations")]
    fn get_node_operations(
        &self,
        from: u64,
        to: u64,
        at: Option<BlockHash>,
    ) -> RpcResult<Vec<NodeOperationJson>>;
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
        from: u64,
        to: u64,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<MetaRecordJson>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        
        let records = api
            .get_meta_records(at_hash, from, to)
            .map_err(|e| ErrorObjectOwned::owned(1, "Runtime error", Some(format!("{:?}", e))))?;
        
        Ok(records
            .into_iter()
            .map(|(timestamp, data)| MetaRecordJson {
                timestamp,
                data: format!("0x{}", hex::encode(data)),
            })
            .collect())
    }
    
    fn get_payload_records(
        &self,
        from: u64,
        to: u64,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<PayloadRecordJson>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        
        let records = api
            .get_payload_records(at_hash, from, to)
            .map_err(|e| ErrorObjectOwned::owned(1, "Runtime error", Some(format!("{:?}", e))))?;
        
        Ok(records
            .into_iter()
            .map(|(timestamp, data)| PayloadRecordJson {
                timestamp,
                data: format!("0x{}", hex::encode(data)),
            })
            .collect())
    }
    
    fn get_node_operations(
        &self,
        from: u64,
        to: u64,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<NodeOperationJson>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        
        let operations = api
            .get_node_operations(at_hash, from, to)
            .map_err(|e| ErrorObjectOwned::owned(1, "Runtime error", Some(format!("{:?}", e))))?;
        
        Ok(operations
            .into_iter()
            .map(|(timestamp, operation_type, data)| NodeOperationJson {
                timestamp,
                operation_type: String::from_utf8_lossy(&operation_type).to_string(),
                data: format!("0x{}", hex::encode(data)),
            })
            .collect())
    }
}
