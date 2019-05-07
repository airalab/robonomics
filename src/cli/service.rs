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
use robonomics_runtime::{self, GenesisConfig, opaque::Block, RuntimeApi};
use transaction_pool::{self, txpool::{Pool as TransactionPool}};
use primitives::{Pair as PairT, ed25519};
use executor::native_executor_instance;
use network::construct_simple_protocol;
use inherents::InherentDataProviders;
use client::{self, LongestChain};
use futures::prelude::*;
use std::sync::Arc;
use log::info;

pub use executor::NativeExecutor;
native_executor_instance!(
    pub Executor,
    robonomics_runtime::api::dispatch,
    robonomics_runtime::native_version,
    include_bytes!("../../runtime/wasm/target/wasm32-unknown-unknown/release/robonomics_runtime.compact.wasm")
);

pub struct NodeConfig<F: substrate_service::ServiceFactory> {
    inherent_data_providers: InherentDataProviders,
    pub grandpa_import_setup: Option<(Arc<grandpa::BlockImportForService<F>>, grandpa::LinkHalfForService<F>)>,
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

                #[cfg(feature = "ros")]
                service.spawn_task(Box::new(ros_integration::start_ros_api(
                    service.network(),
                    service.client(),
                    service.transaction_pool(),
                    service.keystore(),
                    service.on_exit(),
                )));

                #[cfg(feature = "ros")]
                service.spawn_task(Box::new(chain_rpc_ros::start_status_api(
                   service.network(),
                   service.client(),
                   service.transaction_pool(),
                   service.keystore(),
                   service.on_exit(),
                )));

                Ok(service)
            }
        },
        AuthoritySetup = {
            |mut service: Self::FullService, local_key: Option<Arc<ed25519::Pair>>| {
                let (block_import, link_half) = service.config.custom.grandpa_import_setup.take()
                    .expect("Link Half and Block Import are present for Full Services or setup failed before. qed");

                if let Some(ref key) = local_key {
                    info!("Using authority key {}", key.public());
                    let proposer = Arc::new(basic_authorship::ProposerFactory {
                        client: service.client(),
                        transaction_pool: service.transaction_pool(),
                    });

                    let client = service.client();
                    let select_chain = service.select_chain()
                        .ok_or(ServiceError::SelectChainRequired)?;
                    let aura = start_aura(
                        SlotDuration::get_or_compute(&*client)?,
                        key.clone(),
                        client,
                        select_chain,
                        block_import.clone(),
                        proposer,
                        service.network(),
                        service.config.custom.inherent_data_providers.clone(),
                        service.config.force_authoring,
                    )?;
                    service.spawn_task(Box::new(aura.select(service.on_exit()).then(|_| Ok(()))));

                    info!("Running Grandpa session as Authority {}", key.public());
                }


                let local_key = if service.config.disable_grandpa {
                    None
                } else {
                    local_key
                };

                let config = grandpa::Config {
                    local_key,
                    // FIXME #1578 make this available through chainspec
                    gossip_duration: Duration::from_millis(333),
                    justification_period: 4096,
                    name: Some(service.config.name.clone())
                };

                match config.local_key {
                    None => {
                        service.spawn_task(Box::new(grandpa::run_grandpa_observer(
                            config,
                            link_half,
                            service.network(),
                            service.on_exit(),
                        )?));
                    },

                    Some(_) => {
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
                let block_import = Arc::new(block_import);
                let justification_import = block_import.clone();

                config.custom.grandpa_import_setup = Some((block_import.clone(), link_half));

                import_queue::<_, _, ed25519::Pair>(
                    slot_duration,
                    block_import,
                    Some(justification_import),
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
                let block_import = Arc::new(block_import);
                let finality_proof_import = block_import.clone();
                let finality_proof_request_builder = finality_proof_import.create_finality_proof_request_builder();
                import_queue::<_, _, ed25519::Pair>(
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
