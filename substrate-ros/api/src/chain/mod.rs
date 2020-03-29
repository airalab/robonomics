///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life> 
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
use sp_core::{H256, storage::StorageKey};
use sc_client_api::{
    CallExecutor,
    backend::Backend,
    client::BlockBackend,
};
use sp_runtime::{
    traits::{self, Header,},
    generic::BlockId,
};
use sc_client::{
    Client,
//    light::blockchain::RemoteBlockchain,
};
use sc_client::{
    BlockchainEvents, ImportNotifications, FinalityNotifications,
};
use sc_client_api::notifications::StorageEventStream;

mod ros_api;
pub use ros_api::{start_services, start_publishers};

/// Full node chain API.
pub struct FullChain<B, E, Block: traits::Block, RA> {
    /// Substrate client.
    client: Arc<Client<B, E, Block, RA>>,
}

impl<B, E, Block: traits::Block, RA> Clone for FullChain<B, E, Block, RA> {
    fn clone(&self) -> FullChain<B, E, Block, RA> {
        FullChain { client: self.client.clone() }
    }
}

/*
/// Light node chain API.
// TODO: implement light node ROS API.
pub struct LightChain<B, E, Block: traits::Block, RA, F> {
    /// Substrate client.
    client: Arc<Client<B, E, Block, RA>>,
    /// Remote blockchain reference.
    remote_blockchain: Arc<dyn RemoteBlockchain<Block>>,
    /// Remote fetcher reference.
    fetcher: Arc<F>,
}
*/

impl<B, E, Block, RA> FullChain<B, E, Block, RA> where
    Block: traits::Block<Hash=H256> + 'static,
    B: Backend<Block> + Send + Sync + 'static,
    E: CallExecutor<Block> + Send + Sync + 'static,
    RA: Send + Sync + 'static,
{
    pub fn unwrap_or_best(&self, mb_hash: Option<Block::Hash>) -> Block::Hash {
        match mb_hash {
            Some(hash) => hash,
            None => self.client.chain_info().best_hash
        }
    }

    pub fn new(
        client: Arc<Client<B, E, Block, RA>>,
    ) -> Self {
        FullChain {
            client,
        }
    }
}

impl<B, E, Block, RA> BlockchainEvents<Block> for FullChain<B, E, Block, RA> where
    Block: traits::Block<Hash=H256> + 'static,
    B: Backend<Block> + Send + Sync + 'static,
    E: CallExecutor<Block> + Send + Sync + 'static,
    RA: Send + Sync + 'static,
{
    fn import_notification_stream(&self) -> ImportNotifications<Block> {
        self.client.import_notification_stream()
    }

    fn finality_notification_stream(&self) -> FinalityNotifications<Block> {
        self.client.finality_notification_stream()
    }

    fn storage_changes_notification_stream(
        &self,
        filter_keys: Option<&[StorageKey]>,
        child_filter_keys: Option<&[(StorageKey, Option<Vec<StorageKey>>)]>,
    ) -> sp_blockchain::Result<StorageEventStream<Block::Hash>> {
        self.client.storage_changes_notification_stream(filter_keys, child_filter_keys)
    }
}

impl<B, E, Block, RA> ros_api::ChainApi for FullChain<B, E, Block, RA> where
    Block: traits::Block<Hash=H256> + 'static,
    B: Backend<Block> + Send + Sync + 'static,
    E: CallExecutor<Block> + Send + Sync + 'static,
    RA: Send + Sync + 'static,
{
    fn header(&self, hash: Option<ros_api::Hash>) -> Result<String, String> {
        let hash = BlockId::Hash(self.unwrap_or_best(hash.map(Into::into)));
        self.client
            .header(&hash)
            .map(|header| serde_json::to_string(&header).unwrap())
            .map_err(|e| format!("header request error: {}", e))
    }

    fn block(&self, hash: Option<ros_api::Hash>) -> Result<String, String> {
        let hash = BlockId::Hash(self.unwrap_or_best(hash.map(Into::into)));
        self.client
            .block(&hash)
            .map(|block| serde_json::to_string(&block).unwrap())
            .map_err(|e| format!("block request error: {}", e))
    }

    fn block_hash(&self, number: Option<u32>) -> Result<ros_api::Hash, String> {
        let hash = if let Some(num) = number {
            let block_number = BlockId::number(num.into());
            let mb_header = self.client.header(&block_number)
                .map_err(|e| format!("header request error: {}", e))?;
            mb_header.map_or(Default::default(), |h| h.hash())
        } else {
            self.client.chain_info().best_hash
        };
        Ok(hash.into())
    }

    fn finalized_head(&self) -> ros_api::Hash {
        self.client.chain_info().finalized_hash.into()
    }
}
