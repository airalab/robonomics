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
//! Service and ServiceFactory implementation. Specialized wrapper over Substrate service.

use robonomics_primitives::{AccountId, Balance, Block, Index};
use robonomics_protocol::pubsub::gossipsub::PubSub;
use sc_client_api::{BlockBackend, ExecutorProvider};
use sc_consensus_aura::{ImportQueueParams, SlotProportion, StartAuraParams};
pub use sc_executor::NativeElseWasmExecutor;
use sc_finality_grandpa as grandpa;
use sc_network::NetworkService;
use sc_service::{config::Configuration, error::Error as ServiceError, TaskManager};
use sp_api::ConstructRuntimeApi;
use sp_consensus_aura::sr25519::{AuthorityId as AuraId, AuthorityPair as AuraPair};
use sp_runtime::traits::{BlakeTwo256, Block as BlockT};
use std::sync::Arc;
use std::time::Duration;

type FullClient<Runtime, Executor> =
    sc_service::TFullClient<Block, Runtime, NativeElseWasmExecutor<Executor>>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type FullGrandpaBlockImport<Runtime, Executor> =
    grandpa::GrandpaBlockImport<FullBackend, Block, FullClient<Runtime, Executor>, FullSelectChain>;

/// A set of APIs that robonomics-like runtimes must implement.
pub trait RuntimeApiCollection:
    sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
    + sp_api::ApiExt<Block>
    + sp_consensus_aura::AuraApi<Block, AuraId>
    + sp_finality_grandpa::GrandpaApi<Block>
    + sp_block_builder::BlockBuilder<Block>
    + pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
    + frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index>
    + sp_api::Metadata<Block>
    + sp_offchain::OffchainWorkerApi<Block>
    + sp_session::SessionKeys<Block>
where
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

impl<Api> RuntimeApiCollection for Api
where
    Api: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::ApiExt<Block>
        + sp_consensus_aura::AuraApi<Block, AuraId>
        + sp_finality_grandpa::GrandpaApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
        + frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index>
        + sp_api::Metadata<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_session::SessionKeys<Block>,
    <Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

pub fn new_partial<Runtime, Executor>(
    config: &Configuration,
    heartbeat_interval: u64,
) -> Result<
    sc_service::PartialComponents<
        FullClient<Runtime, Executor>,
        FullBackend,
        FullSelectChain,
        sc_consensus::DefaultImportQueue<Block, FullClient<Runtime, Executor>>,
        sc_transaction_pool::FullPool<Block, FullClient<Runtime, Executor>>,
        (
            impl Fn(
                robonomics_rpc::DenyUnsafe,
                sc_rpc::SubscriptionTaskExecutor,
            ) -> Result<jsonrpsee::RpcModule<()>, sc_service::Error>,
            FullGrandpaBlockImport<Runtime, Executor>,
            grandpa::LinkHalf<Block, FullClient<Runtime, Executor>, FullSelectChain>,
            Option<sc_telemetry::Telemetry>,
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
    let telemetry = config
        .telemetry_endpoints
        .clone()
        .filter(|x| !x.is_empty())
        .map(|endpoints| -> Result<_, sc_telemetry::Error> {
            let worker = sc_telemetry::TelemetryWorker::new(16)?;
            let telemetry = worker.handle().new_telemetry(endpoints);
            Ok((worker, telemetry))
        })
        .transpose()?;

    let executor = NativeElseWasmExecutor::<Executor>::new(
        config.wasm_method,
        config.default_heap_pages,
        config.max_runtime_instances,
        config.runtime_cache_size,
    );

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, Runtime, _>(
            &config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
        )?;

    let client = Arc::new(client);
    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager
            .spawn_handle()
            .spawn("telemetry", None, worker.run());
        telemetry
    });

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    let (grandpa_block_import, grandpa_link) = grandpa::block_import(
        client.clone(),
        &(client.clone() as Arc<_>),
        select_chain.clone(),
        telemetry.as_ref().map(|x| x.handle()),
    )?;

    let slot_duration = sc_consensus_aura::slot_duration(&*client)?;
    let import_queue = sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _, _>(
        ImportQueueParams {
            block_import: grandpa_block_import.clone(),
            justification_import: Some(Box::new(grandpa_block_import.clone())),
            client: client.clone(),
            create_inherent_data_providers: move |_, ()| async move {
                let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                let slot =
                    sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                        *timestamp,
                        slot_duration,
                    );

                Ok((timestamp, slot))
            },
            spawner: &task_manager.spawn_essential_handle(),
            can_author_with: sp_consensus::CanAuthorWithNativeVersion::new(
                client.executor().clone(),
            ),
            registry: config.prometheus_registry(),
            check_for_equivocation: Default::default(),
            telemetry: telemetry.as_ref().map(|x| x.handle()),
        },
    )?;

    let (pubsub, pubsub_worker) =
        PubSub::new(Duration::from_millis(heartbeat_interval)).expect("New PubSub");
    task_manager
        .spawn_handle()
        .spawn("pubsub_service", None, pubsub_worker);

    let rpc_extensions_builder = {
        let client = client.clone();
        let pool = transaction_pool.clone();

        move |deny_unsafe, _| {
            let deps = robonomics_rpc::FullDeps {
                client: client.clone(),
                pool: pool.clone(),
                deny_unsafe,
                pubsub: pubsub.clone(),
            };

            robonomics_rpc::create_full(deps).map_err(Into::into)
        }
    };

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        keystore_container,
        select_chain,
        import_queue,
        transaction_pool,
        other: (
            rpc_extensions_builder,
            grandpa_block_import,
            grandpa_link,
            telemetry,
        ),
    })
}

/// Creates a full service from the configuration.
pub fn full_base<Runtime, Executor>(
    mut config: Configuration,
    heartbeat_interval: u64,
) -> Result<
    (
        TaskManager,
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
        keystore_container,
        select_chain,
        transaction_pool,
        other: (rpc_builder, block_import, grandpa_link, mut telemetry),
    } = new_partial(&config, heartbeat_interval)?;

    let warp_sync = Arc::new(grandpa::warp_proof::NetworkProvider::new(
        backend.clone(),
        grandpa_link.shared_authority_set().clone(),
        Vec::default(),
    ));

    let grandpa_protocol_name = grandpa::protocol_standard_name(
        &client
            .block_hash(0)
            .ok()
            .flatten()
            .expect("Genesis block exists; qed"),
        &config.chain_spec,
    );
    config
        .network
        .extra_sets
        .push(grandpa::grandpa_peers_set_config(
            grandpa_protocol_name.clone(),
        ));

    let (network, system_rpc_tx, network_starter) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync: Some(warp_sync),
        })?;

    if config.offchain_worker.enabled {
        sc_service::build_offchain_workers(
            &config,
            task_manager.spawn_handle(),
            client.clone(),
            network.clone(),
        );
    }

    let role = config.role.clone();
    let force_authoring = config.force_authoring;
    let backoff_authoring_blocks =
        Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
    let name = config.network.node_name.clone();
    let enable_grandpa = !config.disable_grandpa;
    let prometheus_registry = config.prometheus_registry().cloned();

    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        config,
        rpc_builder: Box::new(rpc_builder),
        backend: backend.clone(),
        client: client.clone(),
        keystore: keystore_container.sync_keystore(),
        network: network.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        system_rpc_tx,
        telemetry: telemetry.as_mut(),
    })?;

    if role.is_authority() {
        let proposer_factory = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|x| x.handle()),
        );

        let can_author_with =
            sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

        let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

        let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _, _>(
            StartAuraParams {
                slot_duration,
                client: client.clone(),
                select_chain,
                block_import,
                proposer_factory,
                create_inherent_data_providers: move |_, ()| async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                    let slot =
                        sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                            *timestamp,
                            slot_duration,
                        );

                    Ok((timestamp, slot))
                },
                force_authoring,
                backoff_authoring_blocks,
                keystore: keystore_container.sync_keystore(),
                can_author_with,
                sync_oracle: network.clone(),
                justification_sync_link: network.clone(),
                block_proposal_slot_portion: SlotProportion::new(2f32 / 3f32),
                max_block_proposal_slot_portion: None,
                telemetry: telemetry.as_ref().map(|x| x.handle()),
            },
        )?;

        // the AURA authoring task is considered essential, i.e. if it
        // fails we take down the service with it.
        task_manager
            .spawn_essential_handle()
            .spawn_blocking("aura", Some("block-authoring"), aura);
    }

    // if the node isn't actively participating in consensus then it doesn't
    // need a keystore, regardless of which protocol we use below.
    let keystore = if role.is_authority() {
        Some(keystore_container.sync_keystore())
    } else {
        None
    };

    let config = grandpa::Config {
        // FIXME #1578 make this available through chainspec
        gossip_duration: std::time::Duration::from_millis(333),
        justification_period: 512,
        name: Some(name),
        observer_enabled: false,
        local_role: role,
        keystore,
        telemetry: telemetry.as_ref().map(|x| x.handle()),
        protocol_name: grandpa_protocol_name,
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
            voting_rule: grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry,
            shared_voter_state: grandpa::SharedVoterState::empty(),
            telemetry: telemetry.as_ref().map(|x| x.handle()),
        };

        // the GRANDPA voter task is considered infallible, i.e.
        // if it fails we take down the service with it.
        task_manager.spawn_essential_handle().spawn_blocking(
            "grandpa-voter",
            None,
            grandpa::run_grandpa_voter(grandpa_config)?,
        );
    }

    network_starter.start_network();
    Ok((task_manager, client, network, transaction_pool))
}

/// Robonomics chain services.
pub mod robonomics {
    use local_runtime::RuntimeApi;
    use sc_service::{config::Configuration, error::Result, TaskManager};

    pub struct LocalExecutor;
    impl sc_executor::NativeExecutionDispatch for LocalExecutor {
        /// Only enable the benchmarking host functions when we actually want to benchmark.
        #[cfg(feature = "runtime-benchmarks")]
        type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;
        /// Otherwise we only use the default Substrate host functions.
        #[cfg(not(feature = "runtime-benchmarks"))]
        type ExtendHostFunctions = ();

        fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
            local_runtime::api::dispatch(method, data)
        }

        fn native_version() -> sc_executor::NativeVersion {
            local_runtime::native_version()
        }
    }

    /// Create a new Robonomics service.
    pub fn new(config: Configuration, heartbeat_interval: u64) -> Result<TaskManager> {
        super::full_base::<RuntimeApi, LocalExecutor>(config, heartbeat_interval)
            .map(|(task_manager, _, _, _)| task_manager)
    }
}
