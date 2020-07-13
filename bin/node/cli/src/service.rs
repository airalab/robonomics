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
//! Service and ServiceFactory implementation. Specialized wrapper over Substrate service.

/// Native executor for Robonomics runtimes (benchmark enabled).
#[cfg(feature = "frame-benchmarking")]
pub mod executor {
    sc_executor::native_executor_instance!(
        pub Robonomics,
        robonomics_runtime::api::dispatch,
        robonomics_runtime::native_version,
        frame_benchmarking::benchmarking::HostFunctions,
    );

    sc_executor::native_executor_instance!(
        pub Ipci,
        ipci_runtime::api::dispatch,
        ipci_runtime::native_version,
        frame_benchmarking::benchmarking::HostFunctions,
    );
}

/// Native executor for Robonomics runtimes (benchmark disabled).
#[cfg(not(feature = "frame-benchmarking"))]
pub mod executor {
    sc_executor::native_executor_instance!(
        pub Robonomics,
        robonomics_runtime::api::dispatch,
        robonomics_runtime::native_version,
    );

    sc_executor::native_executor_instance!(
        pub Ipci,
        ipci_runtime::api::dispatch,
        ipci_runtime::native_version,
    );
}

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
#[macro_export]
macro_rules! new_full_start {
    ($config:expr, $runtime:ty, $executor:ty) => {{
        use std::sync::Arc;

        let mut import_setup = None;
        let inherent_data_providers = sp_inherents::InherentDataProviders::new();

        let builder = sc_service::ServiceBuilder::new_full::<
            node_primitives::Block,
            $runtime,
            $executor,
        >($config)?
            .with_select_chain(|_config, backend|
                Ok(sc_consensus::LongestChain::new(backend.clone()))
            )?
            .with_transaction_pool(|builder| {
                let pool_api = sc_transaction_pool::FullChainApi::new(
                    builder.client().clone(),
                    builder.prometheus_registry(),
                );
                let pool = sc_transaction_pool::BasicPool::new_full(
                    builder.config().transaction_pool.clone(),
                    Arc::new(pool_api),
                    builder.prometheus_registry(),
                    builder.spawn_handle(),
                    builder.client().clone(),
                );
                Ok(pool)
            })?
            .with_import_queue(|
                _config,
                client,
                mut select_chain,
                _transaction_pool,
                spawn_task_handle,
                prometheus_registry,
            | {
                let select_chain = select_chain
                    .take()
                    .ok_or_else(|| sc_service::Error::SelectChainRequired)?;
                let (grandpa_block_import, grandpa_link) = sc_finality_grandpa::block_import(
                    client.clone(),
                    &(client.clone() as std::sync::Arc<_>),
                    select_chain.clone(),
                )?;
                let justification_import = grandpa_block_import.clone();

                let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
                    sc_consensus_babe::Config::get_or_compute(&*client)?,
                    grandpa_block_import,
                    client.clone(),
                )?;

                let import_queue = sc_consensus_babe::import_queue(
                    babe_link.clone(),
                    babe_block_import.clone(),
                    Some(Box::new(justification_import)),
                    None,
                    client,
                    select_chain,
                    inherent_data_providers.clone(),
                    spawn_task_handle,
                    prometheus_registry,
                )?;

                import_setup = Some((babe_block_import, grandpa_link, babe_link));
                Ok(import_queue)
            },
        )?;

        (builder, import_setup, inherent_data_providers)
    }};
}

/// Creates a full service from the configuration.
#[macro_export]
macro_rules! new_full {
    ($config:expr, $runtime:ty, $executor:ty) => {{
        use futures::prelude::*;
        use sc_network::Event;
        use sc_client_api::call_executor::ExecutorProvider;
        use sp_core::traits::BareCryptoStorePtr;
        use std::sync::Arc;

        let (
            role,
            force_authoring,
            name,
            disable_grandpa,
        ) = (
            $config.role.clone(),
            $config.force_authoring,
            $config.network.node_name.clone(),
            $config.disable_grandpa,
        );
        #[cfg(feature = "ros")]
        let system_info = substrate_ros_api::system::SystemInfo {
            impl_name: $config.impl_name.into(),
            impl_version: $config.impl_version.into(),
            chain_name: $config.chain_spec.name().into(),
            chain_type: $config.chain_spec.chain_type().clone(),
            properties: $config.chain_spec.properties().clone(),
        };

        let (builder, mut import_setup, inherent_data_providers) =
            new_full_start!($config, $runtime, $executor);

        let sc_service::ServiceComponents {
            client, transaction_pool, task_manager, keystore, network, select_chain,
            prometheus_registry, telemetry_on_connect_sinks, ..
        } = builder
            .with_finality_proof_provider(|client, backend| {
                // GenesisAuthoritySetProvider is implemented for StorageAndProofProvider
                let provider = client as Arc<dyn sc_finality_grandpa::StorageAndProofProvider<_, _>>;
                Ok(Arc::new(sc_finality_grandpa::FinalityProofProvider::new(backend, provider)) as _)
            })?
            .build_full()?;

        let (block_import, grandpa_link, babe_link) = import_setup.take()
                .expect("Link Half and Block Import are present for Full Services or setup failed before. qed");

        if let sc_service::config::Role::Authority { .. } = &role {
            let proposer = sc_basic_authorship::ProposerFactory::new(
                client.clone(),
                transaction_pool.clone(),
                prometheus_registry.as_ref(),
            );

            let select_chain = select_chain
                .ok_or(sc_service::Error::SelectChainRequired)?;

            let can_author_with =
                sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

            let babe_config = sc_consensus_babe::BabeParams {
                keystore: keystore.clone(),
                client: client.clone(),
                select_chain,
                env: proposer,
                block_import,
                sync_oracle: network.clone(),
                inherent_data_providers: inherent_data_providers.clone(),
                force_authoring,
                babe_link,
                can_author_with,
            };

            let babe = sc_consensus_babe::start_babe(babe_config)?;
            task_manager.spawn_essential_handle().spawn_blocking("babe-proposer", babe);
        }

        // Spawn authority discovery module.
        if matches!(role, sc_service::config::Role::Authority{..} | sc_service::config::Role::Sentry {..}) {
            let (sentries, authority_discovery_role) = match role {
                sc_service::config::Role::Authority { ref sentry_nodes } => (
                    sentry_nodes.clone(),
                    sc_authority_discovery::Role::Authority (
                        keystore.clone(),
                    ),
                ),
                sc_service::config::Role::Sentry {..} => (
                    vec![],
                    sc_authority_discovery::Role::Sentry,
                ),
                _ => unreachable!("Due to outer matches! constraint; qed.")
            };

            let dht_event_stream = network.event_stream("authority-discovery")
                .filter_map(|e| async move { match e {
                    Event::Dht(e) => Some(e),
                    _ => None,
                }}).boxed();
            let authority_discovery = sc_authority_discovery::AuthorityDiscovery::new(
                client.clone(),
                network.clone(),
                sentries,
                dht_event_stream,
                authority_discovery_role,
                prometheus_registry.clone(),
            );

            task_manager.spawn_handle().spawn("authority-discovery", authority_discovery);
        }

        // if the node isn't actively participating in consensus then it doesn't
        // need a keystore, regardless of which protocol we use below.
        let keystore = if role.is_authority() {
            Some(keystore.clone() as BareCryptoStorePtr)
        } else {
            None
        };

        let config = sc_finality_grandpa::Config {
            // FIXME #1578 make this available through chainspec
            gossip_duration: std::time::Duration::from_millis(333),
            justification_period: 512,
            name: Some(name),
            observer_enabled: false,
            keystore,
            is_authority: role.is_network_authority(),
        };

        let enable_grandpa = !disable_grandpa;
        if enable_grandpa {
            // start the full GRANDPA voter
            // NOTE: non-authorities could run the GRANDPA observer protocol, but at
            // this point the full voter should provide better guarantees of block
            // and vote data availability than the observer. The observer has not
            // been tested extensively yet and having most nodes in a network run it
            // could lead to finality stalls.
            let grandpa_config = sc_finality_grandpa::GrandpaParams {
                config,
                link: grandpa_link,
                network: network.clone(),
                inherent_data_providers: inherent_data_providers.clone(),
                telemetry_on_connect: Some(telemetry_on_connect_sinks.on_connect_stream()),
                voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
                prometheus_registry: prometheus_registry.clone(),
                shared_voter_state: sc_finality_grandpa::SharedVoterState::empty(),
            };

            // the GRANDPA voter task is considered infallible, i.e.
            // if it fails we take down the service with it.
            task_manager.spawn_essential_handle().spawn_blocking(
                "grandpa-voter",
                sc_finality_grandpa::run_grandpa_voter(grandpa_config)?
            );
        } else {
            sc_finality_grandpa::setup_disabled_grandpa(
                client.clone(),
                &inherent_data_providers,
                network.clone(),
            )?;
        }

        #[cfg(feature = "ros")]
        { if rosrust::try_init_with_options("robonomics", false).is_ok() {
            let (substrate_ros_services, publish_task) =
                substrate_ros_api::start(
                    system_info,
                    client.clone(),
                    network.clone(),
                    transaction_pool.clone(),
                    keystore.clone(),
                ).map_err(|e| format!("Substrate ROS: {}", e))?;

            let on_exit = service.on_exit().then(move |_| {
                // Keep ROS services&subscribers alive until on_exit signal reached
                let _ = substrate_ros_services;
                futures::future::ready(())
            });

            let ros_task = futures::future::join(
                publish_task,
                on_exit,
            ).boxed().map(|_| ());

            service.spawn_handle().spawn("substrate-ros", ros_task);
        } else {
            log::warn!("ROS integration disabled because of initialization failure");
        }}

        Ok(task_manager)
    }};
}

/// Creates a light service from the configuration.
#[macro_export]
macro_rules! new_light {
    ($config:expr, $runtime:ty, $executor:ty) => {{
        use std::sync::Arc;

        let inherent_data_providers = sp_inherents::InherentDataProviders::new();

        let sc_service::ServiceComponents { task_manager, .. } =
            sc_service::ServiceBuilder::new_light::<node_primitives::Block, $runtime, $executor>(
                $config,
            )?
            .with_select_chain(|_, backend| Ok(sc_consensus::LongestChain::new(backend.clone())))?
            .with_transaction_pool(|builder| {
                let fetcher = builder.fetcher().ok_or_else(|| {
                    "Trying to start light transaction pool without active fetcher"
                })?;
                let pool_api = sc_transaction_pool::LightChainApi::new(
                    builder.client().clone(),
                    fetcher,
                );
                let pool = sc_transaction_pool::BasicPool::new_light(
                    builder.config().transaction_pool.clone(),
                    Arc::new(pool_api),
                    builder.prometheus_registry(),
                    builder.spawn_handle(),
                );
                Ok(Arc::new(pool))
            })?
            .with_import_queue_and_fprb(|
                _config,
                 client,
                 backend,
                 fetcher,
                 mut select_chain,
                 _tx_pool,
                 spawn_task_handle,
                 registry,
            | {
                let select_chain = select_chain
                    .take()
                    .ok_or_else(|| sc_service::Error::SelectChainRequired)?;
                let fetch_checker = fetcher
                    .map(|fetcher| fetcher.checker().clone())
                    .ok_or_else(|| {
                        "Trying to start light import queue without active fetch checker"
                    })?;
                let grandpa_block_import = sc_finality_grandpa::light_block_import(
                    client.clone(),
                    backend,
                    &(client.clone() as Arc<_>),
                    Arc::new(fetch_checker),
                )?;

                let finality_proof_import = grandpa_block_import.clone();
                let finality_proof_request_builder =
                    finality_proof_import.create_finality_proof_request_builder();

                let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
                    sc_consensus_babe::Config::get_or_compute(&*client)?,
                    grandpa_block_import,
                    client.clone(),
                )?;

                let import_queue = sc_consensus_babe::import_queue(
                    babe_link,
                    babe_block_import,
                    None,
                    Some(Box::new(finality_proof_import)),
                    client,
                    select_chain,
                    inherent_data_providers,
                    spawn_task_handle,
                    registry,
                )?;

                Ok((import_queue, finality_proof_request_builder))
            })?
            .with_finality_proof_provider(|client, backend| {
                // GenesisAuthoritySetProvider is implemented for StorageAndProofProvider
                let provider =
                    client as Arc<dyn sc_finality_grandpa::StorageAndProofProvider<_, _>>;
                Ok(Arc::new(sc_finality_grandpa::FinalityProofProvider::new(
                    backend, provider,
                )) as _)
            })?
            .build_light()?;

        Ok(task_manager)
    }};
}

/// IPCI chain services.
pub mod ipci {
    use sc_service::{config::Configuration, error::Result, TaskManager};

    /// Create a new IPCI service for a full node.
    pub fn new_full(config: Configuration) -> Result<TaskManager> {
        new_full!(config, ipci_runtime::RuntimeApi, super::executor::Ipci)
    }

    /// Create a new IPCI service for a light client.
    pub fn new_light(config: Configuration) -> Result<TaskManager> {
        new_light!(config, ipci_runtime::RuntimeApi, super::executor::Ipci)
    }
}

///  Robonomics chain services.
pub mod robonomics {
    use sc_service::{config::Configuration, error::Result, TaskManager};

    /// Create a new Robonomics service for a full node.
    pub fn new_full(config: Configuration) -> Result<TaskManager> {
        new_full!(
            config,
            robonomics_runtime::RuntimeApi,
            super::executor::Robonomics
        )
    }

    /// Create a new Robonomics service for a light client.
    pub fn new_light(config: Configuration) -> Result<TaskManager> {
        new_light!(
            config,
            robonomics_runtime::RuntimeApi,
            super::executor::Robonomics
        )
    }
}
