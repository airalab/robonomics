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

use codec::{Decode, Encode};
use futures::executor::block_on;
use sc_client::Client;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Error as ClientError;
use sp_core::{traits::BareCryptoStorePtr, H256};
use sp_runtime::{generic, traits};
use sp_session::SessionKeys;
use sp_transaction_pool::{
    error::IntoPoolError, InPoolTransaction, TransactionPool, TransactionSource, TxHash,
};
use std::sync::Arc;

mod ros_api;
pub use ros_api::start_services;

/// Authoring API
pub struct Author<B, E, P, Block: traits::Block, RA> {
    /// Substrate client
    client: Arc<Client<B, E, Block, RA>>,
    /// Transactions pool
    pool: Arc<P>,
    /// The key store.
    keystore: BareCryptoStorePtr,
}

impl<B, E, P, Block: traits::Block, RA> Clone for Author<B, E, P, Block, RA> {
    fn clone(&self) -> Author<B, E, P, Block, RA> {
        Author {
            client: self.client.clone(),
            pool: self.pool.clone(),
            keystore: self.keystore.clone(),
        }
    }
}

impl<B, E, P, Block: traits::Block, RA> Author<B, E, P, Block, RA> {
    /// Create new instance of Authoring API.
    pub fn new(
        client: Arc<Client<B, E, Block, RA>>,
        pool: Arc<P>,
        keystore: BareCryptoStorePtr,
    ) -> Self {
        Author {
            client,
            pool,
            keystore,
        }
    }
}

impl<B, E, P, RA> ros_api::AuthorApi for Author<B, E, P, <P as TransactionPool>::Block, RA>
where
    B: sc_client_api::backend::Backend<<P as TransactionPool>::Block> + Send + Sync + 'static,
    E: sc_client_api::CallExecutor<<P as TransactionPool>::Block> + Clone + Send + Sync + 'static,
    P: TransactionPool<Hash = H256> + Sync + Send + 'static,
    RA: Send + Sync + 'static,
    P::Block: traits::Block<Hash = H256>,
    Client<B, E, P::Block, RA>: ProvideRuntimeApi<P::Block>,
    <Client<B, E, P::Block, RA> as ProvideRuntimeApi<P::Block>>::Api:
        SessionKeys<P::Block, Error = ClientError>,
{
    fn rotate_keys(&self) -> Result<ros_api::Bytes, String> {
        let best_block_hash = self.client.chain_info().best_hash;
        self.client
            .runtime_api()
            .generate_session_keys(&generic::BlockId::Hash(best_block_hash), None)
            .map(Into::into)
            .map_err(|e| format!("{:?}", e))
    }

    fn submit_extrinsic(&self, ext: ros_api::Bytes) -> Result<ros_api::Hash, String> {
        let xt = Decode::decode(&mut &ext[..])
            .map_err(|e| format!("Extrinsic decode failure: {:?}", e))?;
        let best_block_hash = self.client.chain_info().best_hash;
        block_on(self.pool.submit_one(
            &generic::BlockId::hash(best_block_hash),
            TransactionSource::External,
            xt,
        ))
        .map(Into::into)
        .map_err(|e| format!("{:?}", e.into_pool_error()))
    }

    fn pending_extrinsics(&self) -> Vec<ros_api::Bytes> {
        self.pool
            .ready()
            .map(|tx| tx.data().encode().into())
            .collect()
    }

    fn remove_extrinsics(&self, hashes: Vec<ros_api::Hash>) -> Vec<ros_api::Hash> {
        let hashes: Vec<TxHash<P>> = hashes.iter().map(Into::into).collect();
        self.pool
            .remove_invalid(&hashes)
            .into_iter()
            .map(|tx| tx.hash().clone().into())
            .collect()
    }
}
