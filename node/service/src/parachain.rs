///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2024 Robonomics Network <research@robonomics.network>
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
//! Polkadot parachain service implementation.

use cumulus_client_cli::CollatorOptions;
use cumulus_client_consensus_common::{
    ParachainBlockImport as TParachainBlockImport, ParachainConsensus,
};
use cumulus_client_consensus_relay_chain::{
    build_relay_chain_consensus, BuildRelayChainConsensusParams,
};
use cumulus_client_parachain_inherent::ParachainInherentDataProvider;
use cumulus_client_service::{
    build_network, build_relay_chain_interface, prepare_node_config, start_collator,
    start_full_node, BuildNetworkParams, CollatorSybilResistance, StartCollatorParams,
    StartFullNodeParams,
};
use cumulus_primitives_core::ParaId;
use cumulus_relay_chain_interface::RelayChainInterface;
use parity_scale_codec::Encode;
use robonomics_primitives::{AccountId, Balance, Block, Hash, Nonce};

use sc_consensus::ImportQueue;
use sc_executor::{HeapAllocStrategy, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_network::{config::FullNetworkConfiguration, NetworkBlock};
use sc_network_sync::SyncingService;
use sc_service::{Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sp_api::{ApiExt, ConstructRuntimeApi, Metadata};
use sp_keystore::KeystorePtr;
use sp_runtime::traits::BlakeTwo256;
use substrate_prometheus_endpoint::Registry;

use std::sync::Arc;
use std::time::Duration;

type HostFunctions = (
    sp_io::SubstrateHostFunctions,
    frame_benchmarking::benchmarking::HostFunctions,
);
type ParachainClient<RuntimeApi> = TFullClient<Block, RuntimeApi, WasmExecutor<HostFunctions>>;
type ParachainBackend = TFullBackend<Block>;
type ParachainBlockImport<RuntimeApi> =
    TParachainBlockImport<Block, Arc<ParachainClient<RuntimeApi>>, ParachainBackend>;

/// A set of APIs that local runtimes must implement.
pub trait RuntimeApiCollection:
    sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
    + Metadata<Block>
    + ApiExt<Block>
    + sp_offchain::OffchainWorkerApi<Block>
    + sp_block_builder::BlockBuilder<Block>
    + cumulus_primitives_core::CollectCollationInfo<Block>
    + pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
    + frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
    + sp_session::SessionKeys<Block>
where
    sc_client_api::StateBackendFor<ParachainBackend, Block>:
        sc_client_api::StateBackend<BlakeTwo256>,
{
}

impl<Api> RuntimeApiCollection for Api
where
    Api: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + Metadata<Block>
        + ApiExt<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + cumulus_primitives_core::CollectCollationInfo<Block>
        + pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
        + frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
        + sp_session::SessionKeys<Block>,
    sc_client_api::StateBackendFor<ParachainBackend, Block>:
        sc_client_api::StateBackend<BlakeTwo256>,
{
}

/// Build the import queue for the open consensus parachain runtime.
pub fn build_open_import_queue<RuntimeApi>(
    client: Arc<ParachainClient<RuntimeApi>>,
    block_import: ParachainBlockImport<RuntimeApi>,
    config: &Configuration,
    _telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> Result<sc_consensus::DefaultImportQueue<Block>, sc_service::Error>
where
    RuntimeApi: ConstructRuntimeApi<Block, ParachainClient<RuntimeApi>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: RuntimeApiCollection,
{
    let registry = config.prometheus_registry();
    cumulus_client_consensus_relay_chain::import_queue(
        client,
        block_import,
        |_, _| async {
            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
            Ok(timestamp)
        },
        &task_manager.spawn_essential_handle(),
        registry.clone(),
    )
    .map_err(Into::into)
}

/// Build the open set consensus.
pub fn build_open_consensus<RuntimeApi>(
    para_id: ParaId,
    lighthouse_account: AccountId,
    client: Arc<ParachainClient<RuntimeApi>>,
    block_import: ParachainBlockImport<RuntimeApi>,
    prometheus_registry: Option<&Registry>,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
    relay_chain_interface: Arc<dyn RelayChainInterface>,
    transaction_pool: Arc<sc_transaction_pool::FullPool<Block, ParachainClient<RuntimeApi>>>,
    _sync_service: Arc<SyncingService<Block>>,
    _keystore: KeystorePtr,
    _force_authoring: bool,
) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>
where
    RuntimeApi: ConstructRuntimeApi<Block, ParachainClient<RuntimeApi>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: RuntimeApiCollection,
{
    let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
        task_manager.spawn_handle(),
        client.clone(),
        transaction_pool,
        prometheus_registry,
        telemetry.clone(),
    );

    let consensus = build_relay_chain_consensus(BuildRelayChainConsensusParams {
        para_id,
        proposer_factory,
        block_import,
        relay_chain_interface: relay_chain_interface.clone(),
        create_inherent_data_providers: move |_, (relay_parent, validation_data)| {
            let encoded_account = lighthouse_account.encode();
            let relay_chain_interface = relay_chain_interface.clone();
            async move {
                let parachain_inherent = ParachainInherentDataProvider::create_at(
                    relay_parent,
                    &relay_chain_interface,
                    &validation_data,
                    para_id,
                )
                .await;
                let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                let lighthouse =
                    pallet_robonomics_lighthouse::InherentDataProvider(encoded_account);
                let parachain = parachain_inherent.ok_or_else(|| {
                    Box::<dyn std::error::Error + Send + Sync>::from(
                        "Failed to create parachain inherent",
                    )
                })?;
                Ok((timestamp, lighthouse, parachain))
            }
        },
    });

    Ok(consensus)
}

/// Checks that the hardware meets the requirements and print a warning otherwise.
fn warn_if_slow_hardware(hwbench: &sc_sysinfo::HwBench, validator: bool) {
    // Polkadot parachains should generally use these requirements to ensure that
    // will not take longer than expected to import its blocks.
    match frame_benchmarking_cli::SUBSTRATE_REFERENCE_HARDWARE.check_hardware(hwbench, false) {
        Err(err) if validator => {
            log::warn!(
                "⚠️  The hardware does not meet the minimal requirements {} for role 'Authority'.",
                err
            );
        }
        _ => {}
    }
}

/// Partially initialize serivice & deps.
pub fn new_partial<RuntimeApi, BIQ>(
    config: &Configuration,
    build_import_queue: BIQ,
) -> Result<
    PartialComponents<
        ParachainClient<RuntimeApi>,
        ParachainBackend,
        (),
        sc_consensus::DefaultImportQueue<Block>,
        sc_transaction_pool::FullPool<Block, ParachainClient<RuntimeApi>>,
        (
            ParachainBlockImport<RuntimeApi>,
            Option<Telemetry>,
            Option<TelemetryWorkerHandle>,
        ),
    >,
    sc_service::Error,
>
where
    RuntimeApi: ConstructRuntimeApi<Block, ParachainClient<RuntimeApi>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: RuntimeApiCollection,
    BIQ: FnOnce(
        Arc<ParachainClient<RuntimeApi>>,
        ParachainBlockImport<RuntimeApi>,
        &Configuration,
        Option<TelemetryHandle>,
        &TaskManager,
    ) -> Result<sc_consensus::DefaultImportQueue<Block>, sc_service::Error>,
{
    let telemetry = config
        .telemetry_endpoints
        .clone()
        .filter(|x| !x.is_empty())
        .map(|endpoints| -> Result<_, sc_telemetry::Error> {
            let worker = TelemetryWorker::new(16)?;
            let telemetry = worker.handle().new_telemetry(endpoints);
            Ok((worker, telemetry))
        })
        .transpose()?;

    let heap_pages = config
        .executor
        .default_heap_pages
        .map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |h| HeapAllocStrategy::Static {
            extra_pages: h as _,
        });

    let executor = WasmExecutor::<HostFunctions>::builder()
        .with_execution_method(config.executor.wasm_method)
        .with_max_runtime_instances(config.executor.max_runtime_instances)
        .with_runtime_cache_size(config.executor.runtime_cache_size)
        .with_onchain_heap_alloc_strategy(heap_pages)
        .with_offchain_heap_alloc_strategy(heap_pages)
        .build();

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, _>(
            &config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
        )?;
    let client = Arc::new(client);
    let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

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

    let block_import = ParachainBlockImport::new(client.clone(), backend.clone());
    let import_queue = build_import_queue(
        client.clone(),
        block_import.clone(),
        config,
        telemetry.as_ref().map(|telemetry| telemetry.handle()),
        &task_manager,
    )?;

    Ok(sc_service::PartialComponents {
        backend,
        client,
        import_queue,
        keystore_container,
        task_manager,
        transaction_pool,
        select_chain: (),
        other: (block_import, telemetry, telemetry_worker_handle),
    })
}

/// Creates new service from the configuration.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
pub async fn new_service<RuntimeApi, RB, BIQ, BIC>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    para_id: ParaId,
    lighthouse_account: AccountId,
    rpc_ext_builder: RB,
    build_import_queue: BIQ,
    build_consensus: BIC,
    hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<ParachainClient<RuntimeApi>>)>
where
    RuntimeApi: ConstructRuntimeApi<Block, ParachainClient<RuntimeApi>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: RuntimeApiCollection,
    RB: Fn(Arc<ParachainClient<RuntimeApi>>) -> Result<jsonrpsee::RpcModule<()>, sc_service::Error>
        + 'static,
    BIQ: FnOnce(
        Arc<ParachainClient<RuntimeApi>>,
        ParachainBlockImport<RuntimeApi>,
        &Configuration,
        Option<TelemetryHandle>,
        &TaskManager,
    ) -> Result<sc_consensus::DefaultImportQueue<Block>, sc_service::Error>,
    BIC: FnOnce(
        ParaId,
        AccountId,
        Arc<ParachainClient<RuntimeApi>>,
        ParachainBlockImport<RuntimeApi>,
        Option<&Registry>,
        Option<TelemetryHandle>,
        &TaskManager,
        Arc<dyn RelayChainInterface>,
        Arc<sc_transaction_pool::FullPool<Block, ParachainClient<RuntimeApi>>>,
        Arc<SyncingService<Block>>,
        KeystorePtr,
        bool,
    ) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>,
{
    let parachain_config = prepare_node_config(parachain_config);

    let params = new_partial::<RuntimeApi, BIQ>(&parachain_config, build_import_queue)?;

    let (block_import, mut telemetry, telemetry_worker_handle) = params.other;
    let client = params.client.clone();
    let backend = params.backend.clone();
    let mut task_manager = params.task_manager;

    let (relay_chain_interface, collator_key) = build_relay_chain_interface(
        polkadot_config,
        &parachain_config,
        telemetry_worker_handle,
        &mut task_manager,
        collator_options.clone(),
        hwbench.clone(),
    )
    .await
    .map_err(|e| sc_service::Error::Application(Box::new(e) as Box<_>))?;

    let force_authoring = parachain_config.force_authoring;
    let is_authority = parachain_config.role.is_authority();
    let prometheus_registry = parachain_config.prometheus_registry().cloned();
    let transaction_pool = params.transaction_pool.clone();
    let import_queue_service = params.import_queue.service();
    let validator = parachain_config.role.is_authority();

    let net_config = FullNetworkConfiguration::<_, _, sc_network::NetworkWorker<Block, Hash>>::new(
        &parachain_config.network,
        parachain_config
            .prometheus_config
            .as_ref()
            .map(|cfg| cfg.registry.clone()),
    );

    let (network, system_rpc_tx, tx_handler_controller, start_network, sync_service) =
        build_network(BuildNetworkParams {
            parachain_config: &parachain_config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            para_id,
            spawn_handle: task_manager.spawn_handle(),
            relay_chain_interface: relay_chain_interface.clone(),
            import_queue: params.import_queue,
            sybil_resistance_level: CollatorSybilResistance::Resistant,
        })
        .await?;

    let rpc_builder = {
        let client = client.clone();
        let transaction_pool = transaction_pool.clone();

        Box::new(move |_| {
            let deps = robonomics_rpc_core::CoreDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                ext_rpc: rpc_ext_builder(client.clone())?,
            };

            robonomics_rpc_core::create_core_rpc(deps).map_err(Into::into)
        })
    };

    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config: parachain_config,
        keystore: params.keystore_container.keystore(),
        backend: backend.clone(),
        network: network.clone(),
        sync_service: sync_service.clone(),
        system_rpc_tx,
        tx_handler_controller,
        telemetry: telemetry.as_mut(),
    })?;

    if let Some(hwbench) = hwbench {
        sc_sysinfo::print_hwbench(&hwbench);
        if is_authority {
            warn_if_slow_hardware(&hwbench, validator);
        }

        if let Some(ref mut telemetry) = telemetry {
            let telemetry_handle = telemetry.handle();
            task_manager.spawn_handle().spawn(
                "telemetry_hwbench",
                None,
                sc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
            );
        }
    }

    let announce_block = {
        let sync_service = sync_service.clone();
        Arc::new(move |hash, data| sync_service.announce_block(hash, data))
    };

    let relay_chain_slot_duration = Duration::from_secs(6);

    let overseer_handle = relay_chain_interface
        .overseer_handle()
        .map_err(|e| sc_service::Error::Application(Box::new(e)))?;

    if is_authority {
        let parachain_consensus = build_consensus(
            para_id,
            lighthouse_account,
            client.clone(),
            block_import,
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|t| t.handle()),
            &task_manager,
            relay_chain_interface.clone(),
            transaction_pool,
            sync_service.clone(),
            params.keystore_container.keystore(),
            force_authoring,
        )?;

        let spawner = task_manager.spawn_handle();
        let params = StartCollatorParams {
            para_id,
            block_status: client.clone(),
            announce_block,
            client: client.clone(),
            task_manager: &mut task_manager,
            relay_chain_interface,
            spawner,
            parachain_consensus,
            import_queue: import_queue_service,
            collator_key: collator_key.expect("Command line arguments do not allow this. qed"),
            relay_chain_slot_duration,
            recovery_handle: Box::new(overseer_handle),
            sync_service,
        };

        start_collator(params).await?;
    } else {
        let params = StartFullNodeParams {
            client: client.clone(),
            announce_block,
            task_manager: &mut task_manager,
            para_id,
            relay_chain_interface,
            relay_chain_slot_duration,
            import_queue: import_queue_service,
            recovery_handle: Box::new(overseer_handle),
            sync_service,
        };

        start_full_node(params)?;
    }

    start_network.start_network();

    Ok((task_manager, client))
}

/// Start robonomics parachain service
pub async fn start_generic_robonomics_parachain<RuntimeApi>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    para_id: ParaId,
    lighthouse_account: AccountId,
    hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<TaskManager>
where
    RuntimeApi: ConstructRuntimeApi<Block, ParachainClient<RuntimeApi>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: RuntimeApiCollection,
{
    new_service::<RuntimeApi, _, _, _>(
        parachain_config,
        polkadot_config,
        collator_options,
        para_id,
        lighthouse_account,
        |_| Ok(jsonrpsee::RpcModule::new(())),
        build_open_import_queue,
        build_open_consensus,
        hwbench,
    )
    .await
    .map(|r| r.0)
}
