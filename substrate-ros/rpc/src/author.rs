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
use codec::{Encode, Decode};
use primitives::{Bytes, Blake2Hasher, H256};
use runtime_primitives::{generic, traits};
use transaction_pool::{
	txpool::{
		ChainApi as PoolChainApi,
		ExHash as ExHashT,
		Pool,
	},
};
use msgs::substrate_ros_msgs::{
    ExHash, RawExtrinsic,
    SubmitExtrinsic, SubmitExtrinsicRes,
    PendingExtrinsics, PendingExtrinsicsRes,
    RemoveExtrinsic, RemoveExtrinsicRes,
};
use rosrust::api::error::Error;
use crate::traits::RosRpc;

const SUBMIT_SRV_NAME: &str = "/author/submit_extrinsic";
const REMOVE_SRV_NAME: &str = "/author/remove_extrinsic";
const PENDING_SRV_NAME: &str = "/author/pending_extrinsics";

/// Authoring API
pub struct Author<B, E, P, RA> where P: PoolChainApi + Sync + Send + 'static {
    /// Substrate client
    client: Arc<Client<B, E, <P as PoolChainApi>::Block, RA>>,
    /// Transactions pool
    pool: Arc<Pool<P>>,
}

impl<B, E, P, RA> Author<B, E, P, RA> where
	B: client::backend::Backend<<P as PoolChainApi>::Block, Blake2Hasher> + Send + Sync + 'static,
	E: client::CallExecutor<<P as PoolChainApi>::Block, Blake2Hasher> + Send + Sync + 'static,
	P: PoolChainApi<Hash=H256> + Sync + Send + 'static,
	P::Block: traits::Block<Hash=H256>,
	P::Error: 'static,
	RA: Send + Sync + 'static
{
    /// Create new instance of Authoring API.
    pub fn new(
        client: Arc<Client<B, E, <P as PoolChainApi>::Block, RA>>,
        pool: Arc<Pool<P>>,
    ) -> Self {
        Author {
            client,
            pool,
        }
    }

	fn submit_extrinsic(&self, ext: Bytes) -> Result<ExHashT<P>, &str> {
		let xt = Decode::decode(&mut &ext[..]).map_err(|_| "Bad extrinsic format")?;
		let best_block_hash = self.client.info().chain.best_hash;
		self.pool
			.submit_one(&generic::BlockId::hash(best_block_hash), xt)
			.map_err(|_| "Extrinsic pool error")
	}

	fn pending_extrinsics(&self) -> Vec<Bytes> {
		self.pool.ready().map(|tx| tx.data.encode().into()).collect()
	}

	fn remove_extrinsic(&self, hashes: Vec<ExHashT<P>>) -> Vec<ExHashT<P>> {
		self.pool.remove_invalid(&hashes)
			.into_iter()
			.map(|tx| tx.hash.clone())
			.collect()
	}
}

impl<B, E, P, RA> RosRpc for Author<B, E, P, RA> where
	B: client::backend::Backend<<P as PoolChainApi>::Block, Blake2Hasher> + Send + Sync + 'static,
	E: client::CallExecutor<<P as PoolChainApi>::Block, Blake2Hasher> + Send + Sync + 'static,
	P: PoolChainApi<Hash=H256> + Sync + Send + 'static,
	P::Block: traits::Block<Hash=H256>,
	P::Error: 'static,
	RA: Send + Sync + 'static
{
    fn start(api: Arc<Self>) -> Result<Vec<rosrust::Service>, Error> {
        let mut services = vec![];

        let api1 = api.clone();
        services.push(
            rosrust::service::<SubmitExtrinsic, _>(SUBMIT_SRV_NAME, move |req| {
                let mut res = SubmitExtrinsicRes::default();
                match api1.submit_extrinsic(req.extrinsic.data.into()) {
                    Ok(hash) => {
                        res.hash = ExHash::default();
                        res.hash.data = hash.into();
                    },
                    Err(err) => res.error = err.to_string()
                }
                Ok(res)
            })?
        );

        let api2 = api.clone();
        services.push(
            rosrust::service::<PendingExtrinsics, _>(PENDING_SRV_NAME, move |_req| {
                let mut res = PendingExtrinsicsRes::default();
                for xt in api2.pending_extrinsics() {
                    let mut xt_msg = RawExtrinsic::default();
                    for b in xt.iter() { xt_msg.data.push(*b); }
                    res.extrinsics.push(xt_msg);
                }
                Ok(res)
            })?
        );

        let api3 = api.clone();
        services.push(
            rosrust::service::<RemoveExtrinsic, _>(REMOVE_SRV_NAME, move |req| {
                let mut res = RemoveExtrinsicRes::default();
                let hashes = req.extrinsics.iter().map(|h| h.data.into()).collect();
                for xt in api3.remove_extrinsic(hashes) {
                    let mut hash_msg = ExHash::default();
                    hash_msg.data = xt.into();
                    res.extrinsics.push(hash_msg);
                }
                Ok(res)
            })?
        );

        Ok(services)
    }
}
