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
///! Substrate API in ROS namespace.
use futures::future::{join, Future, FutureExt};
use rosrust::api::error::Error;
use sc_client_api::{
    backend::Backend, BlockBackend, BlockchainEvents, ExecutorProvider, StorageProvider,
};
use sc_network::{ExHashT, NetworkService};
pub use sc_rpc::system::helpers::SystemInfo;
use sp_api::{CallApiAt, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_core::H256;
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits;
use sp_session::SessionKeys;
use sp_transaction_pool::TransactionPool;
use std::sync::Arc;

pub mod author;
pub mod chain;
pub mod state;
pub mod system;

pub fn start<BE, H, P, Client>(
    system_info: SystemInfo,
    service_client: Arc<Client>,
    service_network: Arc<NetworkService<P::Block, H>>,
    service_transaction_pool: Arc<P>,
    service_keystore: SyncCryptoStorePtr,
) -> Result<(Vec<rosrust::Service>, impl Future<Output = ()>), Error>
where
    BE: Backend<P::Block> + 'static,
    H: ExHashT + Clone + Sync,
    P: TransactionPool<Hash = H256> + Sync + Send + 'static,
    P::Block: traits::Block<Hash = H256>,
    Client: HeaderBackend<P::Block>
        + BlockBackend<P::Block>
        + BlockchainEvents<P::Block>
        + StorageProvider<P::Block, BE>
        + ExecutorProvider<P::Block>
        + ProvideRuntimeApi<P::Block>
        + CallApiAt<P::Block>
        + Send
        + Sync
        + 'static,
    Client::Api: SessionKeys<P::Block>,
    u64: From<<<P::Block as traits::Block>::Header as traits::Header>::Number>,
{
    let system = system::System::new(system_info, service_network);

    let author = author::Author::new(
        service_client.clone(),
        service_transaction_pool,
        service_keystore,
    );

    let chain = chain::FullChain::new(service_client.clone());

    let state = state::FullState::new(service_client);

    let publishers = join(
        system::start_publishers(system.clone())?,
        chain::start_publishers(chain.clone())?,
    )
    .map(|_| ());

    let services = vec![
        system::start_services(system)?,
        author::start_services(author)?,
        state::start_services(state)?,
        chain::start_services(chain)?,
    ]
    .concat();

    Ok((services, publishers))
}
