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

#![warn(unused_extern_crates)]

use log::warn;
use std::sync::Arc;
use futures::prelude::*;
use sp_api::ConstructRuntimeApi;
use sp_runtime::traits::BlakeTwo256;
use sc_client::LongestChain;
use sc_client_api::ExecutorProvider;
use sc_service::{
    AbstractService, ServiceBuilderCommand,
    TFullClient, TFullBackend, TFullCallExecutor,
    TLightBackend, TLightCallExecutor,
    error::{Error as ServiceError},
    config::Configuration,
};
use node_primitives::{Block, AccountId, Index, Balance};
//pub use polkadot_primitives::parachain::Id as ParaId;
pub use sc_executor::NativeExecutionDispatch;

#[cfg(feature = "frame-benchmarking")]
sc_executor::native_executor_instance!(
    pub RobonomicsExecutor,
    robonomics_runtime::api::dispatch,
    robonomics_runtime::native_version,
    frame_benchmarking::benchmarking::HostFunctions,
);

#[cfg(feature = "frame-benchmarking")]
sc_executor::native_executor_instance!(
    pub IpciExecutor,
    ipci_runtime::api::dispatch,
    ipci_runtime::native_version,
    frame_benchmarking::benchmarking::HostFunctions,
);

#[cfg(not(feature = "frame-benchmarking"))]
sc_executor::native_executor_instance!(
    pub RobonomicsExecutor,
    robonomics_runtime::api::dispatch,
    robonomics_runtime::native_version,
);

#[cfg(not(feature = "frame-benchmarking"))]
sc_executor::native_executor_instance!(
    pub IpciExecutor,
    ipci_runtime::api::dispatch,
    ipci_runtime::native_version,
);

/// A set of APIs that robonomics-like runtimes must implement.
pub trait RuntimeApiCollection<Extrinsic: codec::Codec + Send + Sync + 'static> :
    sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
    + sp_api::ApiExt<Block, Error = sp_blockchain::Error>
    + sp_consensus_babe::BabeApi<Block>
    + sp_block_builder::BlockBuilder<Block>
    + frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index>
    + pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance, Extrinsic>
    + sp_api::Metadata<Block>
    + sp_offchain::OffchainWorkerApi<Block>
    + sp_session::SessionKeys<Block>
    + sp_authority_discovery::AuthorityDiscoveryApi<Block>
where
    Extrinsic: RuntimeExtrinsic,
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{}

impl<Api, Extrinsic> RuntimeApiCollection<Extrinsic> for Api
where
    Api:
    sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
    + sp_api::ApiExt<Block, Error = sp_blockchain::Error>
    + sp_consensus_babe::BabeApi<Block>
    + sp_block_builder::BlockBuilder<Block>
    + frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index>
    + pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance, Extrinsic>
    + sp_api::Metadata<Block>
    + sp_offchain::OffchainWorkerApi<Block>
    + sp_session::SessionKeys<Block>
    + sp_authority_discovery::AuthorityDiscoveryApi<Block>,
    Extrinsic: RuntimeExtrinsic,
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{}

pub trait RuntimeExtrinsic: codec::Codec + Send + Sync + 'static
{}

impl<E> RuntimeExtrinsic for E where E: codec::Codec + Send + Sync + 'static
{}

// We can't use service::TLightClient due to
// Rust bug: https://github.com/rust-lang/rust/issues/43580
type TLightClient<Runtime, Dispatch> = sc_client::Client<
    sc_client::light::backend::Backend<sc_client_db::light::LightStorage<Block>, BlakeTwo256>,
    sc_client::light::call_executor::GenesisCallExecutor<
        sc_client::light::backend::Backend<sc_client_db::light::LightStorage<Block>, BlakeTwo256>,
        sc_client::LocalCallExecutor<
            sc_client::light::backend::Backend<
                sc_client_db::light::LightStorage<Block>,
                BlakeTwo256
            >,
            sc_executor::NativeExecutor<Dispatch>
        >
    >,
    Block,
    Runtime
>;

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
macro_rules! new_full_start {
    ($config:expr, $runtime:ty, $executor:ty) => {{
        let mut import_setup = None;
        let inherent_data_providers = sp_inherents::InherentDataProviders::new();

        let builder = sc_service::ServiceBuilder::new_full::<
            node_primitives::Block, $runtime, $executor
        >($config)?
            .with_select_chain(|_config, backend| {
                Ok(sc_client::LongestChain::new(backend.clone()))
            })?
            .with_transaction_pool(|config, client, _fetcher| {
                let pool_api = sc_transaction_pool::FullChainApi::new(client.clone());
                let pool = sc_transaction_pool::BasicPool::new(config, std::sync::Arc::new(pool_api));
                Ok(pool)
            })?
            .with_import_queue(|_config, client, mut select_chain, _transaction_pool| {
                let select_chain = select_chain.take()
                    .ok_or_else(|| sc_service::Error::SelectChainRequired)?;
                let (grandpa_block_import, grandpa_link) = sc_finality_grandpa::block_import(
                    client.clone(),
                    &(client.clone() as Arc<_>),
                    select_chain
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
                    inherent_data_providers.clone(),
                )?;

                import_setup = Some((babe_block_import, grandpa_link, babe_link));
                Ok(import_queue)
            })?;

        (builder, import_setup, inherent_data_providers)
    }}
}

/// Builds a new IPCI object suitable for chain operations.
pub fn new_ipci_chain_ops(
    config: Configuration,
) -> Result<impl ServiceBuilderCommand<Block=Block>, ServiceError> {
    new_chain_ops::<
        ipci_runtime::RuntimeApi,
        IpciExecutor,
        ipci_runtime::UncheckedExtrinsic,
    >(config)
}

/// Create a new IPCI service for a full node.
pub fn new_ipci_full(
    config: Configuration,
) -> Result<
    impl AbstractService<
        Block = Block,
        RuntimeApi = ipci_runtime::RuntimeApi,
        Backend = TFullBackend<Block>,
        SelectChain = LongestChain<TFullBackend<Block>, Block>,
        CallExecutor = TFullCallExecutor<Block, IpciExecutor>,
    >, ServiceError>
{
    new_full(config)
}

/// Create a new IPCI service for a light client.
pub fn new_ipci_light(
    config: Configuration,
) -> Result<impl AbstractService<
        Block = Block,
        RuntimeApi = ipci_runtime::RuntimeApi,
        Backend = TLightBackend<Block>,
        SelectChain = LongestChain<TLightBackend<Block>, Block>,
        CallExecutor = TLightCallExecutor<Block, IpciExecutor>,
    >, ServiceError>
{
    new_light(config)
}

/// Builds a new Robonomics object suitable for chain operations.
pub fn new_robonomics_chain_ops(
    config: Configuration,
) -> Result<impl ServiceBuilderCommand<Block=Block>, ServiceError> {
    new_chain_ops::<
        robonomics_runtime::RuntimeApi,
        RobonomicsExecutor,
        robonomics_runtime::UncheckedExtrinsic,
    >(config)
}

/// Create a new Robonomics service for a full node.
pub fn new_robonomics_full(
    config: Configuration,
) -> Result<
    impl AbstractService<
        Block = Block,
        RuntimeApi = robonomics_runtime::RuntimeApi,
        Backend = TFullBackend<Block>,
        SelectChain = LongestChain<TFullBackend<Block>, Block>,
        CallExecutor = TFullCallExecutor<Block, RobonomicsExecutor>,
    >, ServiceError>
{
    new_full(config)
}

/// Create a new Robonomics service for a light client.
pub fn new_robonomics_light(
    config: Configuration,
)
    -> Result<impl AbstractService<
        Block = Block,
        RuntimeApi = robonomics_runtime::RuntimeApi,
        Backend = TLightBackend<Block>,
        SelectChain = LongestChain<TLightBackend<Block>, Block>,
        CallExecutor = TLightCallExecutor<Block, RobonomicsExecutor>,
    >, ServiceError>
{
    new_light(config)
}

/// Builds a new object suitable for chain operations.
pub fn new_chain_ops<Runtime, Dispatch, Extrinsic>(
    mut config: Configuration
) -> Result<impl ServiceBuilderCommand<Block=Block>, ServiceError> where
    Runtime: ConstructRuntimeApi<Block, TFullClient<Block, Runtime, Dispatch>> + Send + Sync + 'static,
    Runtime::RuntimeApi:
    RuntimeApiCollection<Extrinsic, StateBackend = sc_client_api::StateBackendFor<TFullBackend<Block>, Block>>,
    Dispatch: NativeExecutionDispatch + 'static,
    Extrinsic: RuntimeExtrinsic,
    <Runtime::RuntimeApi as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
    config.keystore = sc_service::config::KeystoreConfig::InMemory;
    Ok(new_full_start!(config, Runtime, Dispatch).0)
}

/// Creates a full service from the configuration.
pub fn new_full<Runtime, Dispatch, Extrinsic>(
    config: Configuration
) -> Result<
    impl AbstractService<
        Block = Block,
        RuntimeApi = Runtime,
        Backend = TFullBackend<Block>,
        SelectChain = LongestChain<TFullBackend<Block>, Block>,
        CallExecutor = TFullCallExecutor<Block, Dispatch>,
>, ServiceError> where 
    Runtime: ConstructRuntimeApi<Block, TFullClient<Block, Runtime, Dispatch>> + Send + Sync + 'static,
    Runtime::RuntimeApi:
    RuntimeApiCollection<Extrinsic, StateBackend = sc_client_api::StateBackendFor<TFullBackend<Block>, Block>>,
    Dispatch: NativeExecutionDispatch + 'static,
    Extrinsic: RuntimeExtrinsic,
    <Runtime::RuntimeApi as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
    let name = config.network.node_name.clone();
    let role = config.role.clone();
    let disable_grandpa = config.disable_grandpa;
    let force_authoring = config.force_authoring;

    let (builder, mut import_setup, inherent_data_providers) =
        new_full_start!(config, Runtime, Dispatch);

    let service = builder
        .with_finality_proof_provider(|client, backend| {
            // GenesisAuthoritySetProvider is implemented for StorageAndProofProvider
            let provider = client as Arc<dyn sc_finality_grandpa::StorageAndProofProvider<_, _>>;
            Ok(Arc::new(sc_finality_grandpa::FinalityProofProvider::new(backend, provider)) as _)
        })?
        .build()?;

    let (block_import, grandpa_link, babe_link) = import_setup.take()
            .expect("Link Half and Block Import are present for Full Services or setup failed before. qed");

    if let sc_service::config::Role::Authority { sentry_nodes } = &role {
        let proposer = sc_basic_authorship::ProposerFactory::new(
            service.client(),
            service.transaction_pool(),
        );

        let client = service.client();
        let select_chain = service.select_chain()
            .ok_or(sc_service::Error::SelectChainRequired)?;

        let can_author_with =
            sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

        let babe_config = sc_consensus_babe::BabeParams {
            keystore: service.keystore(),
            client,
            select_chain,
            env: proposer,
            block_import,
            sync_oracle: service.network(),
            inherent_data_providers: inherent_data_providers.clone(),
            force_authoring,
            babe_link,
            can_author_with,
        };

        let babe = sc_consensus_babe::start_babe(babe_config)?;
        service.spawn_essential_task("babe-proposer", babe);

        let network = service.network();
        let dht_event_stream = network.event_stream().filter_map(|e| async move { match e {
            sc_network::Event::Dht(e) => Some(e),
            _ => None,
        }}).boxed();
        let authority_discovery = sc_authority_discovery::AuthorityDiscovery::new(
            service.client(),
            network,
            sentry_nodes.clone(),
            service.keystore(),
            dht_event_stream,
            service.prometheus_registry(),
        );

        service.spawn_task("authority-discovery", authority_discovery);
    }

    // if the node isn't actively participating in consensus then it doesn't
    // need a keystore, regardless of which protocol we use below.
    let keystore = if role.is_authority() {
        Some(service.keystore())
    } else {
        None
    };

    let config = sc_finality_grandpa::Config {
        // FIXME #1578 make this available through chainspec
        gossip_duration: std::time::Duration::from_millis(333),
        justification_period: 512,
        name: Some(name),
        observer_enabled: true,
        keystore,
        is_authority: role.is_network_authority(),
    };

    if disable_grandpa {
        sc_finality_grandpa::setup_disabled_grandpa(
            service.client(),
            &inherent_data_providers,
            service.network(),
        )?;
    } else {
        // start the full GRANDPA voter
        // NOTE: non-authorities could run the GRANDPA observer protocol, but at
        // this point the full voter should provide better guarantees of block
        // and vote data availability than the observer. The observer has not
        // been tested extensively yet and having most nodes in a network run it
        // could lead to finality stalls.
        let grandpa_config = sc_finality_grandpa::GrandpaParams {
            config: config,
            link: grandpa_link,
            network: service.network(),
            inherent_data_providers: inherent_data_providers.clone(),
            telemetry_on_connect: Some(service.telemetry_on_connect_stream()),
            voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry: service.prometheus_registry(),
        };
        // the GRANDPA voter task is considered infallible, i.e.
        // if it fails we take down the service with it.
        service.spawn_essential_task(
            "grandpa-voter",
            sc_finality_grandpa::run_grandpa_voter(grandpa_config)?
        );
    }

    #[cfg(feature = "ros")]
    { if rosrust::try_init_with_options("robonomics", false).is_ok() {
        let (robonomics_api, robonomics_ros_services) =
            robonomics_ros_api::start!(service.client());
        service.spawn_task("robonomics-ros", robonomics_api);
    
        let system_info = substrate_ros_api::system::SystemInfo {
            chain_name: config.chain_spec.name().into(),
            impl_name: config.impl_name.into(),
            impl_version: config.impl_version.into(),
            properties: config.chain_spec.properties(),
        };

        let (substrate_ros_services, publish_task) =
            substrate_ros_api::start(
                system_info,
                service.client(),
                service.network(),
                service.transaction_pool(),
                service.keystore(),
            ).map_err(|e| ServiceError::Other(format!("{}", e)))?;

        let on_exit = service.on_exit().then(move |_| {
            // Keep ROS services&subscribers alive until on_exit signal reached
            let _ = substrate_ros_services;
            let _ = robonomics_ros_services; 
            futures::future::ready(())
        });

        let ros_task = futures::future::join(
            publish_task,
            on_exit,
        ).boxed().map(|_| ());

        service.spawn_task("substrate-ros", ros_task);
    } else {
        warn!("ROS integration disabled because of initialization failure");
    } }

    Ok(service)
}

/// Builds a new service for a light client.
pub fn new_light<Runtime, Dispatch, Extrinsic>(
    config: Configuration
) -> Result<impl AbstractService<
        Block = Block,
        RuntimeApi = Runtime,
        Backend = TLightBackend<Block>,
        SelectChain = LongestChain<TLightBackend<Block>, Block>,
        CallExecutor = TLightCallExecutor<Block, Dispatch>,
    >, ServiceError>
where
    Runtime: Send + Sync + 'static,
    Runtime::RuntimeApi:
    RuntimeApiCollection<Extrinsic, StateBackend = sc_client_api::StateBackendFor<TLightBackend<Block>, Block>>,
    Dispatch: NativeExecutionDispatch + 'static,
    Extrinsic: RuntimeExtrinsic,
    Runtime: sp_api::ConstructRuntimeApi<Block, TLightClient<Runtime, Dispatch>>,
{
    let inherent_data_providers = sp_inherents::InherentDataProviders::new();

    sc_service::ServiceBuilder::new_light::<Block, Runtime, Dispatch>(config)?
        .with_select_chain(|_, backend| {
            Ok(sc_client::LongestChain::new(backend.clone()))
        })?
        .with_transaction_pool(|config, client, fetcher| {
            let fetcher = fetcher
                .ok_or_else(|| "Trying to start light transaction pool without active fetcher")?;
            let pool_api = sc_transaction_pool::LightChainApi::new(client.clone(), fetcher.clone());
            let pool = sc_transaction_pool::BasicPool::with_revalidation_type(
                config, Arc::new(pool_api), sc_transaction_pool::RevalidationType::Light,
            );
            Ok(pool)
        })?
        .with_import_queue_and_fprb(|_, client, backend, fetcher, _, _| {
            let fetch_checker = fetcher 
                .map(|fetcher| fetcher.checker().clone())
                .ok_or_else(|| "Trying to start light import queue without active fetch checker")?;
            let grandpa_block_import = sc_finality_grandpa::light_block_import(
                client.clone(),
                backend,
                &(client.clone() as Arc<_>),
                Arc::new(fetch_checker)
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
                inherent_data_providers.clone(),
            )?;

            Ok((import_queue, finality_proof_request_builder))
        })?
        .with_finality_proof_provider(|client, backend| {
            // GenesisAuthoritySetProvider is implemented for StorageAndProofProvider
            let provider = client as Arc<dyn sc_finality_grandpa::StorageAndProofProvider<_, _>>;
            Ok(Arc::new(sc_finality_grandpa::FinalityProofProvider::new(backend, provider)) as _)
        })?
        .build()
}
