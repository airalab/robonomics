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

use futures::prelude::*;
use node_primitives::{AccountId, Balance, Block, Index};
use sc_client_api::{ExecutorProvider, RemoteBackend};
use sc_consensus_babe;
use sc_finality_grandpa::{self as grandpa, FinalityProofProvider as GrandpaFinalityProofProvider};
use sc_network::{Event, NetworkService};
use sc_service::{
    config::{Configuration, Role},
    error::Error as ServiceError,
    RpcHandlers, TaskManager,
};
use sp_api::ConstructRuntimeApi;
use sp_core::traits::BareCryptoStorePtr;
use sp_inherents::InherentDataProviders;
use sp_runtime::traits::{BlakeTwo256, Block as BlockT};
use std::sync::Arc;

type FullClient<Runtime, Executor> = sc_service::TFullClient<Block, Runtime, Executor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type FullGrandpaBlockImport<Runtime, Executor> =
    grandpa::GrandpaBlockImport<FullBackend, Block, FullClient<Runtime, Executor>, FullSelectChain>;
type LightBackend = sc_service::TLightBackendWithHash<Block, BlakeTwo256>;
type LightClient<Runtime, Executor> =
    sc_service::TLightClientWithBackend<Block, Runtime, Executor, LightBackend>;

/// A set of APIs that robonomics-like runtimes must implement.
pub trait RuntimeApiCollection:
    sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
    + sp_api::ApiExt<Block, Error = sp_blockchain::Error>
    + sp_consensus_babe::BabeApi<Block>
    + sp_finality_grandpa::GrandpaApi<Block>
    + sp_block_builder::BlockBuilder<Block>
    + pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
    + frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index>
    + sp_api::Metadata<Block>
    + sp_offchain::OffchainWorkerApi<Block>
    + sp_session::SessionKeys<Block>
    + sp_authority_discovery::AuthorityDiscoveryApi<Block>
where
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

impl<Api> RuntimeApiCollection for Api
where
    Api: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::ApiExt<Block, Error = sp_blockchain::Error>
        + sp_consensus_babe::BabeApi<Block>
        + sp_finality_grandpa::GrandpaApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
        + frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index>
        + sp_api::Metadata<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_session::SessionKeys<Block>
        + sp_authority_discovery::AuthorityDiscoveryApi<Block>,
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

pub fn new_partial<Runtime, Executor>(
    config: &Configuration,
) -> Result<
    sc_service::PartialComponents<
        FullClient<Runtime, Executor>,
        FullBackend,
        FullSelectChain,
        sp_consensus::DefaultImportQueue<Block, FullClient<Runtime, Executor>>,
        sc_transaction_pool::FullPool<Block, FullClient<Runtime, Executor>>,
        (
            impl Fn(node_rpc::DenyUnsafe) -> node_rpc::IoHandler,
            (
                sc_consensus_babe::BabeBlockImport<
                    Block,
                    FullClient<Runtime, Executor>,
                    FullGrandpaBlockImport<Runtime, Executor>,
                >,
                grandpa::LinkHalf<Block, FullClient<Runtime, Executor>, FullSelectChain>,
                sc_consensus_babe::BabeLink<Block>,
            ),
            grandpa::SharedVoterState,
        ),
    >,
    ServiceError,
>
where
    Runtime: ConstructRuntimeApi<Block, FullClient<Runtime, Executor>> + Send + Sync + 'static,
    Runtime::RuntimeApi:
        RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    let (client, backend, keystore, task_manager) =
        sc_service::new_full_parts::<Block, Runtime, Executor>(&config)?;
    let client = Arc::new(client);

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.prometheus_registry(),
        task_manager.spawn_handle(),
        client.clone(),
    );

    let (grandpa_block_import, grandpa_link) = grandpa::block_import(
        client.clone(),
        &(client.clone() as Arc<_>),
        select_chain.clone(),
    )?;
    let justification_import = grandpa_block_import.clone();

    let (block_import, babe_link) = sc_consensus_babe::block_import(
        sc_consensus_babe::Config::get_or_compute(&*client)?,
        grandpa_block_import,
        client.clone(),
    )?;

    let inherent_data_providers = sp_inherents::InherentDataProviders::new();

    let import_queue = sc_consensus_babe::import_queue(
        babe_link.clone(),
        block_import.clone(),
        Some(Box::new(justification_import)),
        None,
        client.clone(),
        select_chain.clone(),
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
    )?;

    let import_setup = (block_import, grandpa_link, babe_link);

    let (rpc_extensions_builder, rpc_setup) = {
        let (_, grandpa_link, babe_link) = &import_setup;

        let shared_authority_set = grandpa_link.shared_authority_set().clone();
        let shared_voter_state = grandpa::SharedVoterState::empty();

        let rpc_setup = shared_voter_state.clone();

        let babe_config = babe_link.config().clone();
        let shared_epoch_changes = babe_link.epoch_changes().clone();

        let client = client.clone();
        let pool = transaction_pool.clone();
        let select_chain = select_chain.clone();
        let keystore = keystore.clone();

        let rpc_extensions_builder = move |deny_unsafe| {
            let deps = node_rpc::FullDeps {
                client: client.clone(),
                pool: pool.clone(),
                select_chain: select_chain.clone(),
                deny_unsafe,
                babe: node_rpc::BabeDeps {
                    babe_config: babe_config.clone(),
                    shared_epoch_changes: shared_epoch_changes.clone(),
                    keystore: keystore.clone(),
                },
                grandpa: node_rpc::GrandpaDeps {
                    shared_voter_state: shared_voter_state.clone(),
                    shared_authority_set: shared_authority_set.clone(),
                },
            };

            node_rpc::create_full(deps)
        };

        (rpc_extensions_builder, rpc_setup)
    };

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        keystore,
        select_chain,
        import_queue,
        transaction_pool,
        inherent_data_providers,
        other: (rpc_extensions_builder, import_setup, rpc_setup),
    })
}

/// Creates a full service from the configuration.
pub fn new_full_base<Runtime, Executor>(
    config: Configuration,
) -> Result<
    (
        TaskManager,
        InherentDataProviders,
        Arc<FullClient<Runtime, Executor>>,
        Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
        Arc<sc_transaction_pool::FullPool<Block, FullClient<Runtime, Executor>>>,
    ),
    ServiceError,
>
where
    Runtime: ConstructRuntimeApi<Block, FullClient<Runtime, Executor>> + Send + Sync + 'static,
    Runtime::RuntimeApi:
        RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore,
        select_chain,
        transaction_pool,
        inherent_data_providers,
        other: (rpc_extensions_builder, import_setup, rpc_setup),
    } = new_partial(&config)?;

    let finality_proof_provider =
        GrandpaFinalityProofProvider::new_for_service(backend.clone(), client.clone());

    let (network, network_status_sinks, system_rpc_tx, network_starter) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: None,
            block_announce_validator_builder: None,
            finality_proof_request_builder: None,
            finality_proof_provider: Some(finality_proof_provider.clone()),
        })?;

    if config.offchain_worker.enabled {
        sc_service::build_offchain_workers(
            &config,
            backend.clone(),
            task_manager.spawn_handle(),
            client.clone(),
            network.clone(),
        );
    }

    let role = config.role.clone();
    let force_authoring = config.force_authoring;
    let name = config.network.node_name.clone();
    let enable_grandpa = !config.disable_grandpa;
    let prometheus_registry = config.prometheus_registry().cloned();
    let telemetry_connection_sinks = sc_service::TelemetryConnectionSinks::default();

    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        config,
        backend: backend.clone(),
        client: client.clone(),
        keystore: keystore.clone(),
        network: network.clone(),
        rpc_extensions_builder: Box::new(rpc_extensions_builder),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        on_demand: None,
        remote_blockchain: None,
        telemetry_connection_sinks: telemetry_connection_sinks.clone(),
        network_status_sinks,
        system_rpc_tx,
    })?;

    let (block_import, grandpa_link, babe_link) = import_setup;
    let shared_voter_state = rpc_setup;

    if let sc_service::config::Role::Authority { .. } = &role {
        let proposer = sc_basic_authorship::ProposerFactory::new(
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
        );

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
        task_manager
            .spawn_essential_handle()
            .spawn_blocking("babe-proposer", babe);
    }

    // Spawn authority discovery module.
    if matches!(role, Role::Authority{..} | Role::Sentry {..}) {
        let (sentries, authority_discovery_role) = match role {
            sc_service::config::Role::Authority { ref sentry_nodes } => (
                sentry_nodes.clone(),
                sc_authority_discovery::Role::Authority(keystore.clone()),
            ),
            sc_service::config::Role::Sentry { .. } => {
                (vec![], sc_authority_discovery::Role::Sentry)
            }
            _ => unreachable!("Due to outer matches! constraint; qed."),
        };

        let dht_event_stream = network
            .event_stream("authority-discovery")
            .filter_map(|e| async move {
                match e {
                    Event::Dht(e) => Some(e),
                    _ => None,
                }
            })
            .boxed();
        let authority_discovery = sc_authority_discovery::AuthorityDiscovery::new(
            client.clone(),
            network.clone(),
            sentries,
            dht_event_stream,
            authority_discovery_role,
            prometheus_registry.clone(),
        );

        task_manager
            .spawn_handle()
            .spawn("authority-discovery", authority_discovery);
    }

    // if the node isn't actively participating in consensus then it doesn't
    // need a keystore, regardless of which protocol we use below.
    let keystore = if role.is_authority() {
        Some(keystore as BareCryptoStorePtr)
    } else {
        None
    };

    let config = grandpa::Config {
        // FIXME #1578 make this available through chainspec
        gossip_duration: std::time::Duration::from_millis(333),
        justification_period: 512,
        name: Some(name),
        observer_enabled: false,
        keystore,
        is_authority: role.is_network_authority(),
    };

    if enable_grandpa {
        // start the full GRANDPA voter
        // NOTE: non-authorities could run the GRANDPA observer protocol, but at
        // this point the full voter should provide better guarantees of block
        // and vote data availability than the observer. The observer has not
        // been tested extensively yet and having most nodes in a network run it
        // could lead to finality stalls.
        let grandpa_config = grandpa::GrandpaParams {
            config,
            link: grandpa_link,
            network: network.clone(),
            inherent_data_providers: inherent_data_providers.clone(),
            telemetry_on_connect: Some(telemetry_connection_sinks.on_connect_stream()),
            voting_rule: grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry,
            shared_voter_state,
        };

        // the GRANDPA voter task is considered infallible, i.e.
        // if it fails we take down the service with it.
        task_manager
            .spawn_essential_handle()
            .spawn_blocking("grandpa-voter", grandpa::run_grandpa_voter(grandpa_config)?);
    } else {
        grandpa::setup_disabled_grandpa(client.clone(), &inherent_data_providers, network.clone())?;
    }

    network_starter.start_network();
    Ok((
        task_manager,
        inherent_data_providers,
        client,
        network,
        transaction_pool,
    ))
}

pub fn new_light_base<Runtime, Executor>(
    config: Configuration,
) -> Result<
    (
        TaskManager,
        Arc<RpcHandlers>,
        Arc<LightClient<Runtime, Executor>>,
        Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
        Arc<
            sc_transaction_pool::LightPool<
                Block,
                LightClient<Runtime, Executor>,
                sc_network::config::OnDemand<Block>,
            >,
        >,
    ),
    ServiceError,
>
where
    Runtime: ConstructRuntimeApi<Block, LightClient<Runtime, Executor>> + Send + Sync + 'static,
    Runtime::RuntimeApi:
        RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<LightBackend, Block>>,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    let (client, backend, keystore, mut task_manager, on_demand) =
        sc_service::new_light_parts::<Block, Runtime, Executor>(&config)?;

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
        config.transaction_pool.clone(),
        config.prometheus_registry(),
        task_manager.spawn_handle(),
        client.clone(),
        on_demand.clone(),
    ));

    let grandpa_block_import = grandpa::light_block_import(
        client.clone(),
        backend.clone(),
        &(client.clone() as Arc<_>),
        Arc::new(on_demand.checker().clone()),
    )?;

    let finality_proof_import = grandpa_block_import.clone();
    let finality_proof_request_builder =
        finality_proof_import.create_finality_proof_request_builder();

    let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
        sc_consensus_babe::Config::get_or_compute(&*client)?,
        grandpa_block_import,
        client.clone(),
    )?;

    let inherent_data_providers = sp_inherents::InherentDataProviders::new();

    let import_queue = sc_consensus_babe::import_queue(
        babe_link,
        babe_block_import,
        None,
        Some(Box::new(finality_proof_import)),
        client.clone(),
        select_chain.clone(),
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
    )?;

    let finality_proof_provider =
        GrandpaFinalityProofProvider::new_for_service(backend.clone(), client.clone());

    let (network, network_status_sinks, system_rpc_tx, network_starter) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: Some(on_demand.clone()),
            block_announce_validator_builder: None,
            finality_proof_request_builder: Some(finality_proof_request_builder),
            finality_proof_provider: Some(finality_proof_provider),
        })?;
    network_starter.start_network();

    if config.offchain_worker.enabled {
        sc_service::build_offchain_workers(
            &config,
            backend.clone(),
            task_manager.spawn_handle(),
            client.clone(),
            network.clone(),
        );
    }

    let light_deps = node_rpc::LightDeps {
        remote_blockchain: backend.remote_blockchain(),
        fetcher: on_demand.clone(),
        client: client.clone(),
        pool: transaction_pool.clone(),
    };

    let rpc_extensions = node_rpc::create_light(light_deps);

    let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        on_demand: Some(on_demand),
        remote_blockchain: Some(backend.remote_blockchain()),
        rpc_extensions_builder: Box::new(sc_service::NoopRpcExtensionBuilder(rpc_extensions)),
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        config,
        keystore,
        backend,
        network_status_sinks,
        system_rpc_tx,
        network: network.clone(),
        telemetry_connection_sinks: sc_service::TelemetryConnectionSinks::default(),
        task_manager: &mut task_manager,
    })?;

    Ok((
        task_manager,
        rpc_handlers,
        client,
        network,
        transaction_pool,
    ))
}

/// IPCI chain services.
pub mod ipci {
    use ipci_runtime::RuntimeApi;
    use sc_service::{config::Configuration, error::Result, TaskManager};

    #[cfg(feature = "frame-benchmarking")]
    sc_executor::native_executor_instance!(
        pub Executor,
        ipci_runtime::api::dispatch,
        ipci_runtime::native_version,
        frame_benchmarking::benchmarking::HostFunctions,
    );

    #[cfg(not(feature = "frame-benchmarking"))]
    sc_executor::native_executor_instance!(
        pub Executor,
        ipci_runtime::api::dispatch,
        ipci_runtime::native_version,
    );

    /// Create a new IPCI service for a full node.
    pub fn new_full(config: Configuration) -> Result<TaskManager> {
        super::new_full_base::<RuntimeApi, Executor>(config)
            .map(|(task_manager, _, _, _, _)| task_manager)
    }

    /// Create a new IPCI service for a light client.
    pub fn new_light(config: Configuration) -> Result<TaskManager> {
        super::new_light_base::<RuntimeApi, Executor>(config)
            .map(|(task_manager, _, _, _, _)| task_manager)
    }
}

///  Robonomics chain services.
pub mod robonomics {
    use robonomics_runtime::RuntimeApi;
    use sc_service::{config::Configuration, error::Result, TaskManager};

    #[cfg(feature = "frame-benchmarking")]
    sc_executor::native_executor_instance!(
        pub Executor,
        robonomics_runtime::api::dispatch,
        robonomics_runtime::native_version,
        frame_benchmarking::benchmarking::HostFunctions,
    );

    #[cfg(not(feature = "frame-benchmarking"))]
    sc_executor::native_executor_instance!(
        pub Executor,
        robonomics_runtime::api::dispatch,
        robonomics_runtime::native_version,
    );

    /// Create a new Robonomics service for a full node.
    pub fn new_full(config: Configuration) -> Result<TaskManager> {
        super::new_full_base::<RuntimeApi, Executor>(config)
            .map(|(task_manager, _, _, _, _)| task_manager)
    }

    pub fn new_light(config: Configuration) -> Result<TaskManager> {
        super::new_light_base::<RuntimeApi, Executor>(config)
            .map(|(task_manager, _, _, _, _)| task_manager)
    }
}
