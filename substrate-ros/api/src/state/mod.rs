///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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

use sc_client_api::{
    backend::{Backend, StorageProvider},
    call_executor::ExecutorProvider,
    CallExecutor,
};
use sp_api::CallApiAt;
use sp_blockchain::HeaderBackend;
use sp_core::{storage, H256};
use sp_runtime::{generic::BlockId, traits};
use sp_state_machine::ExecutionStrategy;
use std::marker::PhantomData;
use std::sync::Arc;

mod ros_api;
pub use ros_api::start_services;

/// Chain state API
pub struct FullState<BE, Client, Block: traits::Block> {
    /// Substrate full client implementation
    client: Arc<Client>,
    /// phantom member to pin block type
    _phantom: PhantomData<(BE, Block)>,
}

impl<BE, Client, Block: traits::Block> Clone for FullState<BE, Client, Block> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<BE, Client, Block> FullState<BE, Client, Block>
where
    Block: traits::Block<Hash = H256> + 'static,
    Client: HeaderBackend<Block> + 'static,
{
    pub fn unwrap_or_best(&self, mb_hash: Option<ros_api::Hash>) -> Block::Hash {
        match mb_hash {
            Some(hash) => hash.into(),
            None => self.client.info().best_hash,
        }
    }

    pub fn new(client: Arc<Client>) -> Self {
        Self {
            client,
            _phantom: PhantomData,
        }
    }
}

impl<BE, Client, Block> ros_api::StateApi for FullState<BE, Client, Block>
where
    BE: Backend<Block>,
    Block: traits::Block<Hash = H256> + 'static,
    Client: StorageProvider<Block, BE>
        + HeaderBackend<Block>
        + ExecutorProvider<Block>
        + CallApiAt<Block>
        + 'static,
{
    fn call(
        &self,
        method: String,
        data: ros_api::Bytes,
        block: Option<ros_api::Hash>,
    ) -> Result<ros_api::Bytes, String> {
        self.client
            .executor()
            .call(
                &BlockId::Hash(self.unwrap_or_best(block)),
                &method,
                &data,
                ExecutionStrategy::NativeElseWasm,
                None,
            )
            .map_err(|e| format!("state error: {}", e))
    }

    fn storage_keys(
        &self,
        key_prefix: ros_api::Bytes,
        block: Option<ros_api::Hash>,
    ) -> Result<Vec<ros_api::Bytes>, String> {
        self.client
            .storage_keys(
                &BlockId::Hash(self.unwrap_or_best(block)),
                &storage::StorageKey(key_prefix),
            )
            .map(|keys| keys.iter().map(|key| key.0.clone()).collect())
            .map_err(|e| format!("state error: {}", e))
    }

    fn storage(
        &self,
        key: ros_api::Bytes,
        block: Option<ros_api::Hash>,
    ) -> Result<Option<ros_api::Bytes>, String> {
        self.client
            .storage(
                &BlockId::Hash(self.unwrap_or_best(block)),
                &storage::StorageKey(key),
            )
            .map_err(|e| format!("state error: {}", e))
            .map(|mb_data| mb_data.map(|data| data.0))
    }

    fn storage_hash(
        &self,
        key: ros_api::Bytes,
        block: Option<ros_api::Hash>,
    ) -> Result<Option<ros_api::Hash>, String> {
        self.client
            .storage_hash(
                &BlockId::Hash(self.unwrap_or_best(block)),
                &storage::StorageKey(key),
            )
            .map_err(|e| format!("state error: {}", e))
            .map(|mb_hash| mb_hash.map(Into::into))
    }

    fn storage_size(
        &self,
        key: ros_api::Bytes,
        block: Option<ros_api::Hash>,
    ) -> Result<Option<u64>, String> {
        self.storage(key, block)
            .map_err(|e| format!("state error: {}", e))
            .map(|x| x.map(|v| v.len() as u64))
    }

    fn runtime_version(&self, block: Option<ros_api::Hash>) -> Result<String, String> {
        self.client
            .runtime_version_at(&BlockId::Hash(self.unwrap_or_best(block)))
            .map_err(|e| format!("state error: {:?}", e))
            .map(|version| serde_json::to_string(&version).unwrap())
    }
}
