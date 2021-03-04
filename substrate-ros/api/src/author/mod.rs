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

use codec::{Decode, Encode};
use futures::executor::block_on;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::H256;
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::{generic, traits};
use sp_session::SessionKeys;
use sp_transaction_pool::{
    error::IntoPoolError, InPoolTransaction, TransactionPool, TransactionSource, TxHash,
};
use std::sync::Arc;

mod ros_api;
pub use ros_api::start_services;

/// Authoring API
pub struct Author<P, Client> {
    /// Substrate client
    client: Arc<Client>,
    /// Transactions pool
    pool: Arc<P>,
    /// The key store.
    keystore: SyncCryptoStorePtr,
}

impl<P, Client> Clone for Author<P, Client> {
    fn clone(&self) -> Self {
        Author {
            client: self.client.clone(),
            pool: self.pool.clone(),
            keystore: self.keystore.clone(),
        }
    }
}

impl<P, Client> Author<P, Client> {
    /// Create new instance of Authoring API.
    pub fn new(client: Arc<Client>, pool: Arc<P>, keystore: SyncCryptoStorePtr) -> Self {
        Author {
            client,
            pool,
            keystore,
        }
    }
}

impl<P, Client> ros_api::AuthorApi for Author<P, Client>
where
    P: TransactionPool<Hash = H256> + Sync + Send + 'static,
    P::Block: traits::Block<Hash = H256>,
    Client: HeaderBackend<P::Block> + ProvideRuntimeApi<P::Block> + Send + Sync + 'static,
    Client::Api: SessionKeys<P::Block>,
{
    fn rotate_keys(&self) -> Result<ros_api::Bytes, String> {
        let best_block_hash = self.client.info().best_hash;
        self.client
            .runtime_api()
            .generate_session_keys(&generic::BlockId::Hash(best_block_hash), None)
            .map(Into::into)
            .map_err(|e| format!("{:?}", e))
    }

    fn submit_extrinsic(&self, ext: ros_api::Bytes) -> Result<ros_api::Hash, String> {
        let xt = Decode::decode(&mut &ext[..])
            .map_err(|e| format!("Extrinsic decode failure: {:?}", e))?;
        let best_block_hash = self.client.info().best_hash;
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
