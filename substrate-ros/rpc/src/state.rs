///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2019 Airalab <research@aira.life> 
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

use std::sync::Arc;
use client::{self, Client};
use primitives::storage::{self, StorageData};
use primitives::{H256, Blake2Hasher};
use runtime_primitives::{
    generic::BlockId, traits::{Block as BlockT}
};
use state_machine::{self, ExecutionStrategy};
use msgs::substrate_ros_msgs::{
    BlockHash, StorageKey,
    StateCall, StateCallRes,
    StorageHash, StorageHashRes,
    StorageKeys, StorageKeysRes,
    StorageQuery, StorageQueryRes,
    StorageSize, StorageSizeRes,
};
use rosrust::api::error::Error;
use crate::traits::RosRpc;

const CALL_SRV_NAME: &str = "/state/call";
const KEYS_SRV_NAME: &str = "/state/keys";
const QUERY_SRV_NAME: &str = "/state/query";
const HASH_SRV_NAME: &str = "/state/hash";
const SIZE_SRV_NAME: &str = "/state/size";

/// Chain API
pub struct State<B, E, Block: BlockT, RA> {
    /// Substrate client
    client: Arc<Client<B, E, Block, RA>>,
}

impl<B, E, Block: BlockT, RA> State<B, E, Block, RA> where
    Block: BlockT<Hash=H256> + 'static,
    B: client::backend::Backend<Block, Blake2Hasher> + Send + Sync + 'static,
    E: client::CallExecutor<Block, Blake2Hasher> + Send + Sync + 'static,
    RA: Send + Sync + 'static
{
    /// Create new instance of Authoring API.
    pub fn new(
        client: Arc<Client<B, E, Block, RA>>,
    ) -> Self {
        State {
            client,
        }
    }

    fn unwrap_or_best(&self, hash: Option<Block::Hash>) -> Block::Hash {
        match hash.into() {
            None => self.client.info().chain.best_hash,
            Some(hash) => hash,
        }
    }

	fn call(&self, method: String, data: Vec<u8>, block: Option<Block::Hash>)
        -> Result<Vec<u8>, String>
    {
		self.client
			.executor()
			.call(
				&BlockId::Hash(self.unwrap_or_best(block)),
				&method, &data, ExecutionStrategy::NativeElseWasm, state_machine::NeverOffchainExt::new(),
			).map_err(|_| "Call error".to_owned())
	}

	fn storage_keys(&self, key_prefix: storage::StorageKey, block: Option<Block::Hash>)
        -> Result<Vec<storage::StorageKey>, String>
    {
		self.client
            .storage_keys(&BlockId::Hash(self.unwrap_or_best(block)), &key_prefix)
            .map_err(|_| "Storage keys error".to_owned())
	}

	fn storage(&self, key: storage::StorageKey, block: Option<Block::Hash>)
        -> Result<Option<StorageData>, String>
    {
		self.client
            .storage(&BlockId::Hash(self.unwrap_or_best(block)), &key)
            .map_err(|_| "Storage query error".to_owned())
	}

	fn storage_hash(&self, key: storage::StorageKey, block: Option<Block::Hash>)
        -> Result<Option<Block::Hash>, String>
    {
		self.client
            .storage_hash(&BlockId::Hash(self.unwrap_or_best(block)), &key)
            .map_err(|_| "Storage hash error".to_owned())
	}

	fn storage_size(&self, key: storage::StorageKey, block: Option<Block::Hash>)
        -> Result<Option<u64>, String>
    {
		self.storage(key, block)
            .map_err(|_| "Storage size error".to_owned())
            .map(|x| x.map(|d| d.0.len() as u64))
	}
}

fn unwrap_hash<Block>(block_msg: BlockHash) -> Option<Block::Hash> where
    Block: BlockT<Hash=H256>
{
    if block_msg.data == [0; 32] {
        None
    } else {
        Some(block_msg.data.into())
    }
}

impl<B, E, Block: BlockT, RA> RosRpc for State<B, E, Block, RA> where
    Block: BlockT<Hash=H256> + 'static,
    B: client::backend::Backend<Block, Blake2Hasher> + Send + Sync + 'static,
    E: client::CallExecutor<Block, Blake2Hasher> + Send + Sync + 'static,
    RA: Send + Sync + 'static
{
    fn start(api: Arc<Self>) -> Result<Vec<rosrust::Service>, Error> {
        let mut services = vec![];

        let api1 = api.clone();
        services.push(
            rosrust::service::<StateCall, _>(CALL_SRV_NAME, move |req| {
                let mut res = StateCallRes::default(); 
                let block = unwrap_hash::<Block>(req.block);
                match api1.call(req.method, req.data, block) {
                    Ok(result) => {
                        res.success = true;
                        res.data = result;
                    },
                    Err(e) => { res.error = e; }
                }
                Ok(res)
            })?
        );

        let api2 = api.clone();
        services.push(
            rosrust::service::<StorageQuery, _>(QUERY_SRV_NAME, move |req| {
                let mut res = StorageQueryRes::default(); 
                let block = unwrap_hash::<Block>(req.block);
                match api2.storage(storage::StorageKey(req.key.data), block) {
                    Ok(result) => {
                        res.success = true;
                        res.data = result.unwrap_or(StorageData(vec![])).0;
                    },
                    Err(e) => { res.error = e; }
                }
                Ok(res)
            })?
        );

        let api3 = api.clone();
        services.push(
            rosrust::service::<StorageSize, _>(SIZE_SRV_NAME, move |req| {
                let mut res = StorageSizeRes::default(); 
                let block = unwrap_hash::<Block>(req.block);
                match api3.storage_size(storage::StorageKey(req.key.data), block) {
                    Ok(result) => {
                        res.success = true;
                        res.size = result.unwrap_or(0);
                    },
                    Err(e) => { res.error = e; }
                }
                Ok(res)
            })?
        );

        let api4 = api.clone();
        services.push(
            rosrust::service::<StorageHash, _>(HASH_SRV_NAME, move |req| {
                let mut res = StorageHashRes::default(); 
                let block = unwrap_hash::<Block>(req.block);
                match api4.storage_hash(storage::StorageKey(req.key.data), block) {
                    Ok(result) => {
                        res.success = true;
                        res.hash.data = result.map(Into::into).unwrap_or([0;32]);
                    },
                    Err(e) => { res.error = e; }
                }
                Ok(res)
            })?
        );

        let api5 = api.clone();
        services.push(
            rosrust::service::<StorageKeys, _>(KEYS_SRV_NAME, move |req| {
                let mut res = StorageKeysRes::default(); 
                let block = unwrap_hash::<Block>(req.block);
                match api5.storage_keys(storage::StorageKey(req.prefix.data), block) {
                    Ok(result) => {
                        res.success = true;
                        for key in result {
                            res.keys.push(StorageKey { data: key.0 });
                        }
                    },
                    Err(e) => { res.error = e; }
                }
                Ok(res)
            })?
        );

        Ok(services)
    }
}
