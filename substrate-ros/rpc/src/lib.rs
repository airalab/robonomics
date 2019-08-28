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
///! Substrate API in ROS namespace.

use network::{specialization::NetworkSpecialization, NetworkService, ExHashT};
use transaction_pool::txpool::{ChainApi as PoolChainApi, Pool};
pub use substrate_rpc::system::helpers::SystemInfo;
use runtime_primitives::traits::Block;
use primitives::{Blake2Hasher, H256};
use rosrust::api::error::Error;
use std::sync::Arc;
use client::Client;

pub mod traits;
pub mod system;
pub mod author;
pub mod chain;
pub mod state;

pub fn start_rpc<B, S, H, F, E, P, A>(
    system_info: SystemInfo,
    service_network: Arc<NetworkService<B, S, H>>,
    service_client: Arc<Client<F, E, <P as PoolChainApi>::Block, A>>,
    service_transaction_pool: Arc<Pool<P>>
) -> Result<(Vec<rosrust::Service>, impl Future<Output=()>), Error> where
    B: Block<Hash=H256>,
    S: NetworkSpecialization<B>,
    H: ExHashT,
    F: client::backend::Backend<<P as PoolChainApi>::Block, Blake2Hasher> + Send + Sync + 'static,
    E: client::CallExecutor<<P as PoolChainApi>::Block, Blake2Hasher> + Send + Sync + 'static,
    P: PoolChainApi<Hash=H256> + Sync + Send + 'static,
    P::Block: Block<Hash=H256>,
    P::Error: 'static,
    A: Send + Sync + 'static
{

    let system = Arc::new(system::System::new(
        system_info,
        service_network,
    ));

    let author = Arc::new(author::Author::new(
        service_client.clone(),
        service_transaction_pool,
    ));

    let chain = Arc::new(chain::Chain::new(
        service_client.clone(),
    ));

    let state = Arc::new(state::State::new(
        service_client,
    ));

    let task = system.start_publishers()?;

    let services = [
        system.start_services()?,
        author.start_services()?,
        state.start_services()?,
        chain.start_services()?,
    ].concat();

    Ok((services, task))
}
