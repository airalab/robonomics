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

use log::info;
use std::sync::Arc;
use grandpa_primitives::{AuthorityPair as GrandpaPair};
use babe::{import_queue, Config};
use babe_primitives::{AuthorityPair as BabePair};
use im_online::sr25519::{AuthorityPair as ImOnlinePair};
use client::{self, LongestChain};
use grandpa::{self, FinalityProofProvider as GrandpaFinalityProofProvider};
use futures03::future::{FutureExt, TryFutureExt};
use futures::prelude::*;
use node_executor::Executor;
use node_runtime::{GenesisConfig, RuntimeApi, types::Block};
use substrate_service::{
    AbstractService, ServiceBuilder, config::Configuration, error::{Error as ServiceError},
};
use transaction_pool::{self, txpool::{Pool as TransactionPool}};
use network::construct_simple_protocol;
use inherents::InherentDataProviders;
use primitives::Pair;

construct_simple_protocol! {
    /// Robonomics protocol attachment for substrate.
    pub struct NodeProtocol where Block = Block { }
}

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
macro_rules! new_full_start {
    ($config:expr) => {{
        let mut import_setup = None;
        let inherent_data_providers = inherents::InherentDataProviders::new();
        let mut tasks_to_spawn = Vec::new();

        let builder = substrate_service::ServiceBuilder::new_full::<
            node_runtime::types::Block, node_runtime::RuntimeApi, node_executor::Executor
        >($config)?
            .with_select_chain(|_config, backend| {
                Ok(client::LongestChain::new(backend.clone()))
            })?
            .with_transaction_pool(|config, client|
                Ok(transaction_pool::txpool::Pool::new(config, transaction_pool::ChainApi::new(client)))
            )?
            .with_import_queue(|_config, client, mut select_chain, transaction_pool| {
                let select_chain = select_chain.take()
                    .ok_or_else(|| substrate_service::Error::SelectChainRequired)?;
                let (block_import, link_half) =
                    grandpa::block_import::<_, _, _, node_runtime::RuntimeApi, _, _>(
                        client.clone(), client.clone(), select_chain
                    )?;
                let justification_import = block_import.clone();

                let (import_queue, babe_link, babe_block_import, pruning_task) = babe::import_queue(
                    babe::Config::get_or_compute(&*client)?,
                    block_import,
                    Some(Box::new(justification_import)),
                    None,
                    client.clone(),
                    client,
                    inherent_data_providers.clone(),
                    Some(transaction_pool)
                )?;

                import_setup = Some((babe_block_import.clone(), link_half, babe_link));
                tasks_to_spawn.push(pruning_task);

                Ok(import_queue)
            })?;

        (builder, import_setup, inherent_data_providers, tasks_to_spawn)
    }}
}

/// Creates a full service from the configuration.
///
/// We need to use a macro because the test suit doesn't work with an opaque service. It expects
/// concrete types instead.
macro_rules! new_full {
    ($config:expr) => {{
        use futures::sync::mpsc;
        use network::DhtEvent;

        let (
            name,
            impl_name,
            impl_version,
            is_authority,
            force_authoring,
            disable_grandpa,
            chain_spec,
        ) = (
            $config.name.clone(),
            $config.impl_name.clone(),
            $config.impl_version.clone(),
            $config.roles.is_authority(),
            $config.force_authoring,
            $config.disable_grandpa,
            $config.chain_spec.clone(),
        );

        let (builder, mut import_setup, inherent_data_providers, tasks_to_spawn) = new_full_start!($config);

        // Dht event channel from the network to the authority discovery module. Use bounded channel to ensure
        // back-pressure. Authority discovery is triggering one event per authority within the current authority set.
        // This estimates the authority set size to be somewhere below 10000 thereby setting the channel buffer size to
        // 10 000.
        let (dht_event_tx, dht_event_rx) = mpsc::channel::<DhtEvent>(10000);

        let service = builder.with_network_protocol(|_| Ok(crate::service::NodeProtocol::new()))?
            .with_finality_proof_provider(|client, backend|
                Ok(Arc::new(grandpa::FinalityProofProvider::new(backend, client)) as _)
            )?
            .with_dht_event_tx(dht_event_tx)?
            .build()?;

        let (block_import, link_half, babe_link) = import_setup.take()
                .expect("Link Half and Block Import are present for Full Services or setup failed before. qed");

        // spawn any futures that were created in the previous setup steps
        tasks_to_spawn.into_iter().for_each(|t| service.spawn_task(t));

        if is_authority {
            let proposer = basic_authorship::ProposerFactory {
                client: service.client(),
                transaction_pool: service.transaction_pool(),
            };

            let client = service.client();
            let select_chain = service.select_chain()
                .ok_or(substrate_service::Error::SelectChainRequired)?;

            let babe_config = babe::BabeParams {
                config: babe::Config::get_or_compute(&*client)?,
                keystore: service.keystore(),
                client,
                select_chain,
                block_import,
                env: proposer,
                sync_oracle: service.network(),
                inherent_data_providers: inherent_data_providers.clone(),
                force_authoring: force_authoring,
                time_source: babe_link,
            };

            let babe = babe::start_babe(babe_config)?;
            service.spawn_essential_task(babe);

            let authority_discovery = authority_discovery::AuthorityDiscovery::new(
                service.client(),
                service.network(),
                dht_event_rx,
            );
            service.spawn_task(authority_discovery);
        }

        let config = grandpa::Config {
            // FIXME #1578 make this available through chainspec
            gossip_duration: std::time::Duration::from_millis(333),
            justification_period: 4096,
            name: Some(name),
            keystore: Some(service.keystore()),
        };

        match (is_authority, disable_grandpa) {
            (false, false) => {
                info!("GRANDPA mode: observer");
                // start the lightweight GRANDPA observer
                service.spawn_task(grandpa::run_grandpa_observer(
                    config,
                    link_half,
                    service.network(),
                    service.on_exit(),
                )?);
            },
            (true, false) => {
                info!("GRANDPA mode: full");
                // start the full GRANDPA voter
                let grandpa_config = grandpa::GrandpaParams {
                    config: config,
                    link: link_half,
                    network: service.network(),
                    inherent_data_providers: inherent_data_providers.clone(),
                    on_exit: service.on_exit(),
                    telemetry_on_connect: Some(service.telemetry_on_connect_stream()),
                };
                service.spawn_task(grandpa::run_grandpa_voter(grandpa_config)?);
            },
            (_, true) => {
                info!("GRANDPA mode: disabled");
                grandpa::setup_disabled_grandpa(
                    service.client(),
                    &inherent_data_providers,
                    service.network(),
                )?;
            },
        }

        #[cfg(feature = "ros")]
        {
            let (api, subs) = ros_robonomics::start_api(
                service.client(),
                service.transaction_pool(),
                babe_key
            );
            service.spawn_task(api.unit_error().boxed().compat());
    
            let system_info = ros_rpc::system::SystemInfo {
                chain_name: chain_spec.name().into(),
                impl_name: impl_name.into(),
                impl_version: impl_version.into(),
                properties: chain_spec.properties(),
            };

            let (srvs, pubs)= ros_rpc::traits::start_services(
                system_info,
                service.network(),
                service.client(),
                service.transaction_pool()
            );
            service.spawn_task(pubs.unit_error().boxed().compat());

            let on_exit = service.on_exit().then(move |_| { let _ = subs; let _ = srvs; Ok(()) });
            service.spawn_task(on_exit);
        }

        Ok((service, inherent_data_providers))
    }}
}

/// Builds a new service for a full client.
pub fn new_full<C: Send + Default + 'static>(
    config: Configuration<C, GenesisConfig>
) -> Result<impl AbstractService, ServiceError> {
    new_full!(config).map(|(service, _)| service)
}

/// Builds a new service for a light client.
pub fn new_light<C: Send + Default + 'static>(
    config: Configuration<C, GenesisConfig>
) -> Result<impl AbstractService, ServiceError> {

    let inherent_data_providers = InherentDataProviders::new();
    let mut tasks_to_spawn = Vec::new();

    let service = ServiceBuilder::new_light::<Block, RuntimeApi, Executor>(config)?
        .with_select_chain(|_config, backend| {
            Ok(LongestChain::new(backend.clone()))
        })?
        .with_transaction_pool(|config, client|
            Ok(TransactionPool::new(config, transaction_pool::ChainApi::new(client)))
        )?
        .with_import_queue_and_fprb(|_config, client, backend, fetcher, _select_chain, transaction_pool| {
            let fetch_checker = fetcher 
                .map(|fetcher| fetcher.checker().clone())
                .ok_or_else(|| "Trying to start light import queue without active fetch checker")?;
            let block_import = grandpa::light_block_import::<_, _, _, RuntimeApi, _>(
                client.clone(), backend, Arc::new(fetch_checker), client.clone()
            )?;

            let finality_proof_import = block_import.clone();
            let finality_proof_request_builder =
                finality_proof_import.create_finality_proof_request_builder();

            let (import_queue, _, _, pruning_task) = import_queue(
                Config::get_or_compute(&*client)?,
                block_import,
                None,
                Some(Box::new(finality_proof_import)),
                client.clone(),
                client,
                inherent_data_providers.clone(),
                Some(transaction_pool)
            )?;

            tasks_to_spawn.push(pruning_task);

            Ok((import_queue, finality_proof_request_builder))
        })?
        .with_network_protocol(|_| Ok(NodeProtocol::new()))?
        .with_finality_proof_provider(|client, backend|
            Ok(Arc::new(GrandpaFinalityProofProvider::new(backend, client)) as _)
        )?
        .build()?;

    // spawn any futures that were created in the previous setup steps
    tasks_to_spawn.into_iter().for_each(|t| service.spawn_task(t));

    Ok(service)
}
