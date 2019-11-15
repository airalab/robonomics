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
//! Service and ServiceFactory implementation. Specialized wrapper over Substrate service.

#![warn(unused_extern_crates)]

use std::time::Duration;

use substrate_service::{
    FactoryFullConfiguration, LightComponents, FullComponents, FullBackend,
    FullClient, LightClient, LightBackend, FullExecutor, LightExecutor,
    TelemetryOnConnect, construct_service_factory,
    error::{Error as ServiceError}
};
use futures03::future::{FutureExt, TryFutureExt};
use babe::{import_queue, start_babe, BabeImportQueue, Config};
use babe_primitives::AuthorityPair as BabePair;
use grandpa::{self, FinalityProofProvider as GrandpaFinalityProofProvider};
use grandpa_primitives::AuthorityPair as GrandpaPair;
use robonomics_runtime::{self, GenesisConfig, types::Block, RuntimeApi};
use transaction_pool::{self, txpool::{Pool as TransactionPool}};
use executor::native_executor_instance;
use network::construct_simple_protocol;
use inherents::InherentDataProviders;
use client::{self, LongestChain};
use primitives::{Pair, sr25519};
use futures::prelude::*;
use std::sync::Arc;
use log::info;
use ipfs_api::IpfsClient;

use futures03::channel::mpsc;
use futures03_util::stream::StreamExt;
use futures03_util::future::ready;

pub use executor::NativeExecutor;
native_executor_instance!(
    pub Executor,
    robonomics_runtime::api::dispatch,
    robonomics_runtime::native_version,
    robonomics_runtime::WASM_BINARY
);

pub struct NodeConfig<F: substrate_service::ServiceFactory> {
    inherent_data_providers: InherentDataProviders,
    /// GRANDPA and BABE connection to import block.
    // FIXME #1134 rather than putting this on the config, let's have an actual 
    // intermediate setup state
    pub import_setup: Option<(
        BabeBlockImportForService<F>,
        grandpa::LinkHalfForService<F>,
        babe::BabeLink,
    )>,
    /// Tasks that were created by previous setup steps and should be spawned.
    pub tasks_to_spawn: Option<Vec<Box<dyn Future<Item = (), Error = ()> + Send>>>,
}

impl<F> Default for NodeConfig<F> where F: substrate_service::ServiceFactory {
    fn default() -> NodeConfig<F> {
        NodeConfig {
            inherent_data_providers: InherentDataProviders::new(),
            import_setup: None,
            tasks_to_spawn: None,
        }
    }
}

type BabeBlockImportForService<F> = babe::BabeBlockImport<
    FullBackend<F>,
    FullExecutor<F>,
    <F as crate::ServiceFactory>::Block,
    grandpa::BlockImportForService<F>,
    <F as crate::ServiceFactory>::RuntimeApi,
    client::Client<
        FullBackend<F>,
        FullExecutor<F>,
        <F as crate::ServiceFactory>::Block,
        <F as crate::ServiceFactory>::RuntimeApi
    >,
>;

construct_simple_protocol! {
    /// Robonomics protocol attachment for substrate.
    pub struct Protocol where Block = Block { }
}

construct_service_factory! {
    struct Factory {
        Block = Block,
        ConsensusPair = BabePair,
        FinalityPair = GrandpaPair,
        RuntimeApi = RuntimeApi,
        NetworkProtocol = Protocol { |config| Ok(Protocol::new()) },
        RuntimeDispatch = Executor,
        FullTransactionPoolApi = transaction_pool::ChainApi<client::Client<FullBackend<Self>, FullExecutor<Self>, Block, RuntimeApi>, Block>
            { |config, client| Ok(TransactionPool::new(config, transaction_pool::ChainApi::new(client))) },
        LightTransactionPoolApi = transaction_pool::ChainApi<client::Client<LightBackend<Self>, LightExecutor<Self>, Block, RuntimeApi>, Block>
            { |config, client| Ok(TransactionPool::new(config, transaction_pool::ChainApi::new(client))) },
        Genesis = GenesisConfig,
        Configuration = NodeConfig<Self>,
        FullService = FullComponents<Self> {
            |config: FactoryFullConfiguration<Self>| {
                let service = FullComponents::<Factory>::new(config)?;

                #[cfg(feature = "ros")]
                {
                    let key = service.authority_key().unwrap();
                    let (api, api_subs) = ros_robonomics::start_api(
                            service.client(),
                            service.transaction_pool(),
                            sr25519::Pair::from_seed_slice(&key.to_raw_vec()).unwrap(),
                        );
                    service.spawn_task(Box::new(api.unit_error().boxed().compat()));

                    let ipfs_client = Arc::new(IpfsClient::default());

                    let (fut, liability_engine_services, liability_engine_subscribers) = ros_robonomics::start_liability_engine(ipfs_client).unwrap();
                    service.spawn_task(Box::new(fut.unit_error().boxed().compat()));

                    let system_info = ros_rpc::system::SystemInfo {
                        chain_name: service.config.chain_spec.name().into(),
                        impl_name: service.config.impl_name.into(),
                        impl_version: service.config.impl_version.into(),
                        properties: service.config.chain_spec.properties(),
                    };

                    let (srvs, publishers)= ros_rpc::traits::start_services(system_info.clone(), service.network(), service.client(), service.transaction_pool());
                    service.spawn_task(Box::new(Box::new(publishers.unit_error().boxed().compat())));

                    let on_exit = service.on_exit().then(move |_| {
                        liability_engine_services;
                        liability_engine_subscribers;
                        srvs;
                        api_subs;
                        Ok(())
                    });
                    service.spawn_task(Box::new(on_exit));
                }

                Ok(service)
            }
        },
        AuthoritySetup = {
            |mut service: Self::FullService| {
                let (block_import, link_half, babe_link) = service.config.custom.import_setup.take()
                    .expect("Link Half and Block Import are present for Full Services or setup failed before. qed");

                // spawn any futures that were created in the previous setup steps
                if let Some(tasks) = service.config.custom.tasks_to_spawn.take() {
                    for task in tasks {
                        service.spawn_task(
                            task.select(service.on_exit())
                                .map(|_| ())
                                .map_err(|_| ())
                        );
                    }
                }

                if let Some(babe_key) = service.authority_key() {
                    info!("Using BABE key {}", babe_key.public());

                    let proposer = basic_authorship::ProposerFactory {
                        client: service.client(),
                        transaction_pool: service.transaction_pool(),
                    };

                    let client = service.client();
                    let select_chain = service.select_chain()
                        .ok_or(ServiceError::SelectChainRequired)?;

                    let babe_config = babe::BabeParams {
                        config: Config::get_or_compute(&*client)?,
                        local_key: Arc::new(babe_key),
                        client,
                        select_chain,
                        block_import,
                        env: proposer,
                        sync_oracle: service.network(),
                        inherent_data_providers: service.config.custom.inherent_data_providers.clone(),
                        force_authoring: service.config.force_authoring,
                        time_source: babe_link,
                    };

                    let babe = start_babe(babe_config)?;
                    let select = babe.select(service.on_exit()).then(|_| Ok(()));
                    service.spawn_task(Box::new(select));
                }

                let grandpa_key = if service.config.disable_grandpa {
                    None
                } else {
                    service.fg_authority_key()
                };

                let config = grandpa::Config {
                    local_key: grandpa_key.map(Arc::new),
                    // FIXME #1578 make this available through chainspec
                    gossip_duration: Duration::from_millis(333),
                    justification_period: 4096,
                    name: Some(service.config.name.clone())
                };

                match config.local_key {
                    None if !service.config.grandpa_voter => {
                        service.spawn_task(Box::new(grandpa::run_grandpa_observer(
                            config,
                            link_half,
                            service.network(),
                            service.on_exit(),
                        )?));
                    },
                    // Either config.local_key is set, or user forced voter service via `--grandpa-voter` flag.
                    _ => {
                        let telemetry_on_connect = TelemetryOnConnect {
                            telemetry_connection_sinks: service.telemetry_on_connect_stream(),
                        };

                        let grandpa_config = grandpa::GrandpaParams {
                            config: config,
                            link: link_half,
                            network: service.network(),
                            inherent_data_providers: service.config.custom.inherent_data_providers.clone(),
                            on_exit: service.on_exit(),
                            telemetry_on_connect: Some(telemetry_on_connect),
                        };

                        service.spawn_task(Box::new(grandpa::run_grandpa_voter(grandpa_config)?));
                    },
                }

                Ok(service)
            }
        },
        LightService = LightComponents<Self>
            { |config| <LightComponents<Factory>>::new(config) },
        FullImportQueue = BabeImportQueue<Self::Block>
            { |config: &mut FactoryFullConfiguration<Self>, client: Arc<FullClient<Self>>, select_chain: Self::SelectChain| {
                let (block_import, link_half) =
                    grandpa::block_import::<_, _, _, RuntimeApi, FullClient<Self>, _>(
                        client.clone(), client.clone(), select_chain
                    )?;
                let justification_import = block_import.clone();

                let (import_queue, babe_link, babe_block_import, pruning_task) = import_queue(
                    Config::get_or_compute(&*client)?,
                    block_import,
                    Some(Box::new(justification_import)),
                    None,
                    client.clone(),
                    client,
                    config.custom.inherent_data_providers.clone(),
                )?;

                config.custom.import_setup = Some((babe_block_import.clone(), link_half, babe_link));
                config.custom.tasks_to_spawn = Some(vec![Box::new(pruning_task)]);

                Ok(import_queue)
            }},
        LightImportQueue = BabeImportQueue<Self::Block>
            { |config: &mut FactoryFullConfiguration<Self>, client: Arc<LightClient<Self>>| {
                #[allow(deprecated)]
                let fetch_checker = client.backend().blockchain().fetcher()
                    .upgrade()
                    .map(|fetcher| fetcher.checker().clone())
                    .ok_or_else(|| "Trying to start light import queue without active fetch checker")?;
                let block_import = grandpa::light_block_import::<_, _, _, RuntimeApi, LightClient<Self>>(
                    client.clone(), Arc::new(fetch_checker), client.clone()
                )?;

                let finality_proof_import = block_import.clone();
                let finality_proof_request_builder = finality_proof_import.create_finality_proof_request_builder();

                // FIXME: pruning task isn't started since light client doesn't do `AuthoritySetup`.
                let (import_queue, ..) = import_queue(
                    Config::get_or_compute(&*client)?,
                    block_import,
                    None,
                    Some(Box::new(finality_proof_import)),
                    client.clone(),
                    client,
                    config.custom.inherent_data_providers.clone(),
                )?;

                Ok((import_queue, finality_proof_request_builder))
            }
        },
        SelectChain = LongestChain<FullBackend<Self>, Self::Block>
            { |config: &FactoryFullConfiguration<Self>, client: Arc<FullClient<Self>>| {
                #[allow(deprecated)]
                Ok(LongestChain::new(client.backend().clone()))
            }
        },
        FinalityProofProvider = { |client: Arc<FullClient<Self>>| {
            Ok(Some(Arc::new(GrandpaFinalityProofProvider::new(client.clone(), client)) as _))
        }},
    }
}
