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
//! Polkadot collator service implementation.

use codec::Encode;
use cumulus_client_cli::CollatorOptions;
use cumulus_client_consensus_common::ParachainConsensus;
use cumulus_client_consensus_relay_chain::{
    build_relay_chain_consensus, BuildRelayChainConsensusParams,
};
use cumulus_client_network::BlockAnnounceValidator;
use cumulus_client_service::{
    prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_parachain_inherent::ParachainInherentData;
use cumulus_relay_chain_inprocess_interface::build_inprocess_relay_chain;
use cumulus_relay_chain_interface::{RelayChainError, RelayChainInterface, RelayChainResult};
use cumulus_relay_chain_rpc_interface::RelayChainRPCInterface;
use hex_literal::hex;
use polkadot_service::CollatorPair;
use robonomics_primitives::{AccountId, Balance, Block, Hash, Index};
//use robonomics_protocol::pubsub::gossipsub::PubSub;
pub use sc_executor::NativeElseWasmExecutor;
use sc_network::NetworkService;
use sc_service::{Configuration, Role, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{TelemetryHandle, TelemetryWorkerHandle};
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::BlakeTwo256;
use sp_trie::PrefixedMemoryDB;
use std::sync::Arc;
use std::time::Duration;
use substrate_prometheus_endpoint::Registry;

async fn build_relay_chain_interface(
    polkadot_config: Configuration,
    parachain_config: &Configuration,
    telemetry_worker_handle: Option<TelemetryWorkerHandle>,
    task_manager: &mut TaskManager,
    collator_options: CollatorOptions,
) -> RelayChainResult<(
    Arc<(dyn RelayChainInterface + 'static)>,
    Option<CollatorPair>,
)> {
    match collator_options.relay_chain_rpc_url {
        Some(relay_chain_url) => Ok((
            Arc::new(RelayChainRPCInterface::new(relay_chain_url).await?) as Arc<_>,
            None,
        )),
        None => build_inprocess_relay_chain(
            polkadot_config,
            parachain_config,
            telemetry_worker_handle,
            task_manager,
            None,
        ),
    }
}

fn new_partial<RuntimeApi, Executor, BIQ>(
    config: &Configuration,
    build_import_queue: BIQ,
) -> Result<
    sc_service::PartialComponents<
        TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        TFullBackend<Block>,
        (),
        sc_consensus::import_queue::BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
        sc_transaction_pool::FullPool<
            Block,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
        (
            Option<sc_telemetry::Telemetry>,
            Option<sc_telemetry::TelemetryWorkerHandle>,
        ),
    >,
    sc_service::Error,
>
where
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    RuntimeApi: sp_api::ConstructRuntimeApi<
            Block,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        > + Send
        + Sync
        + 'static,
    RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<
            Block,
            StateBackend = sc_client_api::StateBackendFor<TFullBackend<Block>, Block>,
        > + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
    BIQ: FnOnce(
        Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
        &Configuration,
        Option<TelemetryHandle>,
        &TaskManager,
    ) -> Result<
        sc_consensus::DefaultImportQueue<
            Block,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
        sc_service::Error,
    >,
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

    let import_queue = build_import_queue(
        client.clone(),
        config,
        telemetry.as_ref().map(|telemetry| telemetry.handle()),
        &task_manager,
    )?;

    let params = sc_service::PartialComponents {
        backend,
        client,
        import_queue,
        keystore_container,
        task_manager,
        transaction_pool,
        select_chain: (),
        other: (telemetry, telemetry_worker_handle),
    };

    Ok(params)
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
pub async fn start_node_impl<RuntimeApi, Executor, BIQ, BIC>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    id: polkadot_primitives::v2::Id,
    lighthouse_account: Option<AccountId>,
    build_import_queue: BIQ,
    build_consensus: BIC,
    _heartbeat_interval: u64,
) -> sc_service::error::Result<TaskManager>
where
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    RuntimeApi: sp_api::ConstructRuntimeApi<
            Block,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        > + Send
        + Sync
        + 'static,
    RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<
            Block,
            StateBackend = sc_client_api::StateBackendFor<TFullBackend<Block>, Block>,
        > + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + cumulus_primitives_core::CollectCollationInfo<Block>
        + pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
        + substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
    BIQ: FnOnce(
        Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
        &Configuration,
        Option<TelemetryHandle>,
        &TaskManager,
    ) -> Result<
        sc_consensus::DefaultImportQueue<
            Block,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
        sc_service::Error,
    >,
    BIC: FnOnce(
        polkadot_primitives::v2::Id,
        Option<AccountId>,
        Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
        Option<&Registry>,
        Option<TelemetryHandle>,
        &TaskManager,
        Arc<dyn RelayChainInterface>,
        Arc<
            sc_transaction_pool::FullPool<
                Block,
                TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
            >,
        >,
        Arc<NetworkService<Block, Hash>>,
        SyncCryptoStorePtr,
        bool,
    ) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>,
{
    if matches!(parachain_config.role, Role::Light) {
        return Err("Light client not supported!".into());
    }

    let parachain_config = prepare_node_config(parachain_config);

    let params = new_partial::<RuntimeApi, Executor, BIQ>(&parachain_config, build_import_queue)?;

    let (mut telemetry, telemetry_worker_handle) = params.other;
    let client = params.client.clone();
    let backend = params.backend.clone();
    let mut task_manager = params.task_manager;

    let (relay_chain_interface, collator_key) = build_relay_chain_interface(
        polkadot_config,
        &parachain_config,
        telemetry_worker_handle,
        &mut task_manager,
        collator_options.clone(),
    )
    .await
    .map_err(|e| match e {
        RelayChainError::ServiceError(polkadot_service::Error::Sub(x)) => x,
        s => s.to_string().into(),
    })?;
    let block_announce_validator = BlockAnnounceValidator::new(relay_chain_interface.clone(), id);

    let prometheus_registry = parachain_config.prometheus_registry().cloned();
    let force_authoring = parachain_config.force_authoring;
    let is_authority = parachain_config.role.is_authority();
    let transaction_pool = params.transaction_pool.clone();
    let import_queue = cumulus_client_service::SharedImportQueue::new(params.import_queue);
    let (network, system_rpc_tx, start_network) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &parachain_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue: import_queue.clone(),
            block_announce_validator_builder: Some(Box::new(|_| {
                Box::new(block_announce_validator)
            })),
            warp_sync: None,
        })?;

    let rpc_client = client.clone();
    let rpc_pool = transaction_pool.clone();

    /*
    let (pubsub, pubsub_worker) =
        PubSub::new(Duration::from_millis(heartbeat_interval)).expect("New PubSub");
    task_manager
        .spawn_handle()
        .spawn("pubsub_parachain", None, pubsub_worker);
    */

    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder: Box::new(move |deny_unsafe, _| {
            let deps = robonomics_rpc::FullDeps {
                client: rpc_client.clone(),
                pool: rpc_pool.clone(),
                deny_unsafe,
                //pubsub: pubsub.clone(),
            };

            robonomics_rpc::create_full(deps).map_err(Into::into)
        }),
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config: parachain_config,
        keystore: params.keystore_container.sync_keystore(),
        backend: backend.clone(),
        network: network.clone(),
        system_rpc_tx,
        telemetry: telemetry.as_mut(),
    })?;

    let announce_block = {
        let network = network.clone();
        Arc::new(move |hash, data| network.announce_block(hash, data))
    };

    let relay_chain_slot_duration = Duration::from_secs(6);

    if is_authority {
        let parachain_consensus = build_consensus(
            id,
            lighthouse_account,
            client.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|t| t.handle()),
            &task_manager,
            relay_chain_interface.clone(),
            transaction_pool,
            network,
            params.keystore_container.sync_keystore(),
            force_authoring,
        )?;

        let spawner = task_manager.spawn_handle();
        let params = StartCollatorParams {
            para_id: id,
            block_status: client.clone(),
            import_queue,
            announce_block,
            client: client.clone(),
            task_manager: &mut task_manager,
            relay_chain_interface,
            spawner,
            parachain_consensus,
            collator_key: collator_key.expect("Command line arguments do not allow this. qed"),
            relay_chain_slot_duration,
        };

        start_collator(params).await?;
    } else {
        let params = StartFullNodeParams {
            client: client.clone(),
            collator_options,
            announce_block,
            task_manager: &mut task_manager,
            para_id: id,
            relay_chain_interface,
            relay_chain_slot_duration,
            import_queue,
        };

        start_full_node(params)?;
    }

    start_network.start_network();

    Ok(task_manager)
}

/// Build the import queue for the open consensus parachain runtime.
pub fn build_open_import_queue<RuntimeApi, Executor>(
    client: Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
    config: &Configuration,
    _telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> Result<
    sc_consensus::DefaultImportQueue<
        Block,
        TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
    >,
    sc_service::Error,
>
where
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    RuntimeApi: sp_api::ConstructRuntimeApi<
            Block,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        > + Send
        + Sync
        + 'static,
    RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<
            Block,
            StateBackend = sc_client_api::StateBackendFor<TFullBackend<Block>, Block>,
        > + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + cumulus_primitives_core::CollectCollationInfo<Block>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
{
    let registry = config.prometheus_registry();
    cumulus_client_consensus_relay_chain::import_queue(
        client.clone(),
        client.clone(),
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
pub fn build_open_consensus<RuntimeApi, Executor>(
    para_id: polkadot_primitives::v2::Id,
    lighthouse_account: Option<AccountId>,
    client: Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
    prometheus_registry: Option<&Registry>,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
    relay_chain_interface: Arc<dyn RelayChainInterface>,
    transaction_pool: Arc<
        sc_transaction_pool::FullPool<
            Block,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        >,
    >,
    _sync_oracle: Arc<NetworkService<Block, Hash>>,
    _keystore: SyncCryptoStorePtr,
    _force_authoring: bool,
) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>
where
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    RuntimeApi: sp_api::ConstructRuntimeApi<
            Block,
            TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
        > + Send
        + Sync
        + 'static,
    RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<
            Block,
            StateBackend = sc_client_api::StateBackendFor<TFullBackend<Block>, Block>,
        > + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + cumulus_primitives_core::CollectCollationInfo<Block>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
{
    let account = lighthouse_account.unwrap_or(
        // Treasury by default
        hex!["6d6f646c70792f74727372790000000000000000000000000000000000000000"].into(),
    );

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
        block_import: client.clone(),
        relay_chain_interface: relay_chain_interface.clone(),
        create_inherent_data_providers: move |_, (relay_parent, validation_data)| {
            let encoded_account = account.encode();
            let relay_chain_interface = relay_chain_interface.clone();
            async move {
                let parachain_inherent = ParachainInherentData::create_at(
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
