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
use consensus::{import_queue, start_aura, AuraImportQueue, SlotDuration};
use grandpa::{self, FinalityProofProvider as GrandpaFinalityProofProvider};
use robonomics_runtime::{self, GenesisConfig, opaque::Block, RuntimeApi, AuraPair};
use transaction_pool::{self, txpool::{Pool as TransactionPool}};
use executor::native_executor_instance;
use network::construct_simple_protocol;
use inherents::InherentDataProviders;
use client::{self, LongestChain};
use futures::prelude::*;
use primitives::Pair;
use std::sync::Arc;
use log::info;

pub use executor::NativeExecutor;
native_executor_instance!(
    pub Executor,
    robonomics_runtime::api::dispatch,
    robonomics_runtime::native_version,
    robonomics_runtime::WASM_BINARY
);

pub struct NodeConfig<F: substrate_service::ServiceFactory> {
    inherent_data_providers: InherentDataProviders,
    pub grandpa_import_setup: Option<(grandpa::BlockImportForService<F>, grandpa::LinkHalfForService<F>)>,
}

impl<F> Default for NodeConfig<F> where F: substrate_service::ServiceFactory {
    fn default() -> NodeConfig<F> {
        NodeConfig {
            grandpa_import_setup: None,
            inherent_data_providers: InherentDataProviders::new(),
        }
    }
}

construct_simple_protocol! {
    /// Robonomics protocol attachment for substrate.
    pub struct Protocol where Block = Block { }
}

construct_service_factory! {
    struct Factory {
        Block = Block,
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

                /*
                #[cfg(feature = "ros")]
                service.spawn_task(Box::new(ros_robonomics::start_ros_api(
                    service.network(),
                    service.client(),
                    service.transaction_pool(),
                    service.on_exit(),
                )));
                */

                #[cfg(feature = "ros")]
                {
                    let author = ros_rpc::author::Author::new(
                        service.client(),
                        service.transaction_pool(),
                    );

                    let chain = ros_rpc::chain::Chain::new(
                        service.client(),
                    );

                    let _services = vec![
                        ros_rpc::traits::RosRpc::start(Arc::new(author)).unwrap(),
                        ros_rpc::traits::RosRpc::start(Arc::new(chain)).unwrap(),
                    ];

                    let on_exit = service.on_exit().then(move |_| {_services; Ok(())});
                    service.spawn_task(Box::new(on_exit));
                }

                Ok(service)
            }
        },
        AuthoritySetup = {
            |mut service: Self::FullService| {
                let (block_import, link_half) = service.config.custom.grandpa_import_setup.take()
                    .expect("Link Half and Block Import are present for Full Services or setup failed before. qed");

                if let Some(aura_key) = service.authority_key::<AuraPair>() {
                    info!("Using aura key {}", aura_key.public());

                    let proposer = Arc::new(basic_authorship::ProposerFactory {
                        client: service.client(),
                        transaction_pool: service.transaction_pool(),
                    });

                    let client = service.client();
                    let select_chain = service.select_chain()
                        .ok_or(ServiceError::SelectChainRequired)?;

                    let aura = start_aura(
                        SlotDuration::get_or_compute(&*client)?,
                        Arc::new(aura_key),
                        client,
                        select_chain,
                        block_import,
                        proposer,
                        service.network(),
                        service.config.custom.inherent_data_providers.clone(),
                        service.config.force_authoring,
                    )?;
                    let select = aura.select(service.on_exit()).then(|_| Ok(()));
                    service.spawn_task(Box::new(select));
                }

                let grandpa_key = if service.config.disable_grandpa {
                    None
                } else {
                    service.authority_key::<grandpa_primitives::AuthorityPair>()
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
        FullImportQueue = AuraImportQueue<Self::Block>
            { |config: &mut FactoryFullConfiguration<Self>, client: Arc<FullClient<Self>>, select_chain: Self::SelectChain| {
                let slot_duration = SlotDuration::get_or_compute(&*client)?;
                let (block_import, link_half) =
                    grandpa::block_import::<_, _, _, RuntimeApi, FullClient<Self>, _>(
                        client.clone(), client.clone(), select_chain
                    )?;
                let justification_import = block_import.clone();

                config.custom.grandpa_import_setup = Some((block_import.clone(), link_half));

                import_queue::<_, _, AuraPair>(
                    slot_duration,
                    Box::new(block_import),
                    Some(Box::new(justification_import)),
                    None,
                    None,
                    client,
                    config.custom.inherent_data_providers.clone(),
                ).map_err(Into::into)
            }},
        LightImportQueue = AuraImportQueue<Self::Block>
            { |config: &mut FactoryFullConfiguration<Self>, client: Arc<LightClient<Self>>| {
                #[allow(deprecated)]
                let fetch_checker = client.backend().blockchain().fetcher()
                    .upgrade()
                    .map(|fetcher| fetcher.checker().clone())
                    .ok_or_else(|| "Trying to start light import queue without active fetch checker")?;
                let block_import = grandpa::light_block_import::<_, _, _, RuntimeApi, LightClient<Self>>(
                    client.clone(), Arc::new(fetch_checker), client.clone()
                )?;
                let block_import = Box::new(block_import);
                let finality_proof_import = block_import.clone();
                let finality_proof_request_builder = finality_proof_import.create_finality_proof_request_builder();
                import_queue::<_, _, AuraPair>(
                    SlotDuration::get_or_compute(&*client)?,
                    block_import,
                    None,
                    Some(finality_proof_import),
                    Some(finality_proof_request_builder),
                    client,
                    config.custom.inherent_data_providers.clone(),
                ).map_err(Into::into)
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
