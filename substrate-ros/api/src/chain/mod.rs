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
    client::BlockBackend, notifications::StorageEventStream, BlockchainEvents,
    FinalityNotifications, ImportNotifications,
};
use sp_blockchain::HeaderBackend;
use sp_core::{storage::StorageKey, H256};
use sp_runtime::{
    generic::BlockId,
    traits::{self, Header},
};
use std::marker::PhantomData;
use std::sync::Arc;

mod ros_api;
pub use ros_api::{start_publishers, start_services};

/// Full node chain API.
pub struct FullChain<Client, Block: traits::Block> {
    /// Substrate client.
    client: Arc<Client>,
    /// phantom member to pin block type
    _phantom: PhantomData<Block>,
}

impl<Client, Block: traits::Block> Clone for FullChain<Client, Block> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            _phantom: PhantomData,
        }
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

impl<Client, Block> FullChain<Client, Block>
where
    Block: traits::Block + 'static,
    Client: HeaderBackend<Block> + 'static,
{
    pub fn unwrap_or_best(&self, mb_hash: Option<Block::Hash>) -> Block::Hash {
        match mb_hash {
            Some(hash) => hash,
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

impl<Client, Block> BlockchainEvents<Block> for FullChain<Client, Block>
where
    Block: traits::Block<Hash = H256> + 'static,
    Client: BlockchainEvents<Block> + 'static,
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
        self.client
            .storage_changes_notification_stream(filter_keys, child_filter_keys)
    }
}

impl<Client, Block> ros_api::ChainApi for FullChain<Client, Block>
where
    Block: traits::Block<Hash = H256> + 'static,
    Client: BlockBackend<Block> + HeaderBackend<Block> + 'static,
{
    fn header(&self, hash: Option<ros_api::Hash>) -> Result<String, String> {
        let hash = BlockId::Hash(self.unwrap_or_best(hash.map(Into::into)));
        self.client
            .header(hash)
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
            let mb_header = self
                .client
                .header(block_number)
                .map_err(|e| format!("header request error: {}", e))?;
            mb_header.map_or(Default::default(), |h| h.hash())
        } else {
            self.client.info().best_hash
        };
        Ok(hash.into())
    }

    fn finalized_head(&self) -> ros_api::Hash {
        self.client.info().finalized_hash.into()
    }
}
