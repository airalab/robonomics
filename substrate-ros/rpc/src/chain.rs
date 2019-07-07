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
use primitives::{H256, Blake2Hasher};
use runtime_primitives::generic::{BlockId, SignedBlock};
use runtime_primitives::traits::{Block as BlockT, Header};
use msgs::substrate_ros_msgs::{
    GetBlock, GetBlockRes,
    GetBlockHash, GetBlockHashRes,
    GetBlockHeader, GetBlockHeaderRes,
    GetBestHead, GetBestHeadRes,
    GetFinalizedHead, GetFinalizedHeadRes,
};
use rosrust::api::error::Error;
use crate::traits::RosRpc;

const BLOCK_SRV_NAME: &str = "/chain/block";
const BLOCK_HASH_SRV_NAME: &str = "/chain/block_hash";
const BLOCK_HEADER_SRV_NAME: &str = "/chain/block_header";
const BEST_HEAD_SRV_NAME: &str = "/chain/best_head";
const FINALIZED_HEAD_SRV_NAME: &str = "/chain/finalized_head";

/// Chain API
pub struct Chain<B, E, Block: BlockT, RA> {
    /// Substrate client
    client: Arc<Client<B, E, Block, RA>>,
}

impl<B, E, Block: BlockT, RA> Chain<B, E, Block, RA> where
    Block: BlockT<Hash=H256> + 'static,
    B: client::backend::Backend<Block, Blake2Hasher> + Send + Sync + 'static,
    E: client::CallExecutor<Block, Blake2Hasher> + Send + Sync + 'static,
    RA: Send + Sync + 'static
{
    /// Create new instance of Authoring API.
    pub fn new(
        client: Arc<Client<B, E, Block, RA>>,
    ) -> Self {
        Chain {
            client,
        }
    }

    fn unwrap_or_best(&self, hash: Option<Block::Hash>) -> Block::Hash {
        match hash.into() {
            None => self.client.info().chain.best_hash,
            Some(hash) => hash,
        }
    }

    fn header(&self, hash: Option<Block::Hash>) -> Option<Block::Header> {
        let hash = self.unwrap_or_best(hash);
        self.client.header(&BlockId::Hash(hash)).unwrap()
    }

    fn block(&self, hash: Option<Block::Hash>) -> Option<SignedBlock<Block>>
    {
        let hash = self.unwrap_or_best(hash);
        self.client.block(&BlockId::Hash(hash)).unwrap()
    }

    fn block_hash(&self, number: Option<u32>) -> Option<Block::Hash> where 
    {
        match number {
            None => Some(self.client.info().chain.best_hash),
            Some(num) => self.client.header(&BlockId::number(num.into())).unwrap().map(|h| h.hash()),
        }
    }

    fn finalized_head(&self) -> Block::Hash {
        self.client.info().chain.finalized_hash
    }
}

impl<B, E, Block: BlockT, RA> RosRpc for Chain<B, E, Block, RA> where
    Block: BlockT<Hash=H256> + 'static,
    B: client::backend::Backend<Block, Blake2Hasher> + Send + Sync + 'static,
    E: client::CallExecutor<Block, Blake2Hasher> + Send + Sync + 'static,
    RA: Send + Sync + 'static
{
    fn start(api: Arc<Self>) -> Result<Vec<rosrust::Service>, Error> {
        let mut services = vec![];

        let api1 = api.clone();
        services.push(
            rosrust::service::<GetBlockHeader, _>(BLOCK_HEADER_SRV_NAME, move |req| {
                let mut res = GetBlockHeaderRes::default(); 
                let hash = if req.hash.data == [0; 32] {
                    None
                } else {
                    Some(req.hash.data.into())
                };
                res.header_json = serde_json::to_string(&api1.header(hash)).unwrap();
                Ok(res)
            })?
        );

        let api2 = api.clone();
        services.push(
            rosrust::service::<GetBlock, _>(BLOCK_SRV_NAME, move |req| {
                let mut res = GetBlockRes::default();
                let hash = if req.hash.data == [0; 32] {
                    None
                } else {
                    Some(req.hash.data.into())
                };
                res.block_json = serde_json::to_string(&api2.block(hash)).unwrap();
                Ok(res)
            })?
        );

        let api3 = api.clone();
        services.push(
            rosrust::service::<GetBlockHash, _>(BLOCK_HASH_SRV_NAME, move |req| {
                let mut res = GetBlockHashRes::default();
                res.hash.data = api3.block_hash(Some(req.number)).unwrap().into();
                Ok(res)
            })?
        );

        let api4 = api.clone();
        services.push(
            rosrust::service::<GetBestHead, _>(BEST_HEAD_SRV_NAME, move |_| {
                let mut res = GetBestHeadRes::default();
                res.hash.data = api4.block_hash(None).unwrap().into();
                Ok(res)
            })?
        );

        let api5 = api.clone();
        services.push(
            rosrust::service::<GetFinalizedHead, _>(FINALIZED_HEAD_SRV_NAME, move |_| {
                let mut res = GetFinalizedHeadRes::default();
                res.hash.data = api5.finalized_head().into();
                Ok(res)
            })?
        );

        Ok(services)
    }
}
