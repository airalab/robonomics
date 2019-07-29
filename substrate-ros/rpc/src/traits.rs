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
use rosrust::Service;
use rosrust::api::error::Error;

pub use substrate_rpc::system::helpers::SystemInfo;

use client::Client;
use transaction_pool::{
    txpool::{
        ChainApi as PoolChainApi,
        Pool,
    },
};
use network::{
    specialization::NetworkSpecialization,
    NetworkService, ExHashT,
};
use runtime_primitives::traits;
use primitives::{Bytes, Blake2Hasher, H256};
use futures::prelude::*;

pub fn start_services<B, S, H, F, E, P, A> (system_info: SystemInfo,
                      service_network: Arc<NetworkService<B, S, H>>,
                      service_client: Arc<Client<F, E, <P as PoolChainApi>::Block, A>>,
                      service_transaction_pool: Arc<Pool<P>>
                    )
    -> (Vec<rosrust::Service>, impl Future<Output=()>) where
    B: traits::Block<Hash=H256>,
    S: NetworkSpecialization<B>,
    H: ExHashT,
    F: client::backend::Backend<<P as PoolChainApi>::Block, Blake2Hasher> + Send + Sync + 'static,
    E: client::CallExecutor<<P as PoolChainApi>::Block, Blake2Hasher> + Send + Sync + 'static,
    P: PoolChainApi<Hash=H256> + Sync + Send + 'static,
    P::Block: traits::Block<Hash=H256>,
    P::Error: 'static,
    A: Send + Sync + 'static
{

    let system = Arc::new(crate::system::System::new(
        system_info,
        service_network,
    ));

    let author = Arc::new(crate::author::Author::new(
        service_client.clone(),
        service_transaction_pool,
    ));

    let chain = Arc::new(crate::chain::Chain::new(
        service_client.clone(),
    ));

    let state = Arc::new(crate::state::State::new(
        service_client,
    ));

    let services = [
        crate::system::System::start(system.clone()).unwrap(),
        crate::author::Author::start(author.clone()).unwrap(),
        crate::state::State::start(state.clone()).unwrap(),
        crate::chain::Chain::start(chain.clone()).unwrap(),
    ].concat();

    let system_publisher_future = crate::system::start_system_publishers(system);

    (services, system_publisher_future)
}

pub trait RosRpc {
    fn start(api: Arc<Self>) -> Result<Vec<Service>, Error>;
}