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
use cumulus_client_consensus_aura::{
    build_aura_consensus, BuildAuraConsensusParams, SlotProportion,
};
use cumulus_client_consensus_common::ParachainConsensus;
use cumulus_client_consensus_relay_chain::{
    build_relay_chain_consensus, BuildRelayChainConsensusParams,
};
use cumulus_client_network::build_block_announce_validator;
use cumulus_client_service::{
    prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_parachain_inherent::ParachainInherentData;
use robonomics_primitives::{AccountId, Block, Hash, Index};
use sc_client_api::ExecutorProvider;
use sc_network::NetworkService;
use sc_service::{Role, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::TelemetryHandle;
use sp_consensus::SlotData;
use sp_consensus_aura::{sr25519::AuthorityId as AuraId, AuraApi};
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::BlakeTwo256;
use sp_trie::PrefixedMemoryDB;
use std::sync::Arc;
use substrate_frame_rpc_system::{FullSystem, SystemApi};
use substrate_prometheus_endpoint::Registry;

fn new_partial<RuntimeApi, Executor, BIQ>(
    config: &sc_service::Configuration,
    build_import_queue: BIQ,
) -> Result<
    sc_service::PartialComponents<
        TFullClient<Block, RuntimeApi, Executor>,
        TFullBackend<Block>,
        (),
        sc_consensus::import_queue::BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
        sc_transaction_pool::FullPool<Block, TFullClient<Block, RuntimeApi, Executor>>,
        (
            Option<sc_telemetry::Telemetry>,
            Option<sc_telemetry::TelemetryWorkerHandle>,
        ),
    >,
    sc_service::Error,
>
where
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    RuntimeApi: sp_api::ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, Executor>>
        + Send
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
        Arc<TFullClient<Block, RuntimeApi, Executor>>,
        &sc_service::Configuration,
        Option<TelemetryHandle>,
        &TaskManager,
    ) -> Result<
        sc_consensus::DefaultImportQueue<Block, TFullClient<Block, RuntimeApi, Executor>>,
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

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, Executor>(
            &config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
        )?;
    let client = Arc::new(client);
    let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager.spawn_handle().spawn("telemetry", worker.run());
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
    parachain_config: sc_service::Configuration,
    polkadot_config: sc_service::Configuration,
    id: polkadot_primitives::v0::Id,
    lighthouse_account: Option<AccountId>,
    build_import_queue: BIQ,
    build_consensus: BIC,
) -> sc_service::error::Result<TaskManager>
where
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    RuntimeApi: sp_api::ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, Executor>>
        + Send
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
        + substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
    BIQ: FnOnce(
        Arc<TFullClient<Block, RuntimeApi, Executor>>,
        &sc_service::Configuration,
        Option<TelemetryHandle>,
        &TaskManager,
    ) -> Result<
        sc_consensus::DefaultImportQueue<Block, TFullClient<Block, RuntimeApi, Executor>>,
        sc_service::Error,
    >,
    BIC: FnOnce(
        polkadot_primitives::v0::Id,
        Option<AccountId>,
        Arc<TFullClient<Block, RuntimeApi, Executor>>,
        Option<&Registry>,
        Option<TelemetryHandle>,
        &TaskManager,
        &polkadot_service::NewFull<polkadot_service::Client>,
        Arc<sc_transaction_pool::FullPool<Block, TFullClient<Block, RuntimeApi, Executor>>>,
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
    let relay_chain_full_node =
        cumulus_client_service::build_polkadot_full_node(polkadot_config, telemetry_worker_handle)
            .map_err(|e| match e {
                polkadot_service::Error::Sub(x) => x,
                s => format!("{}", s).into(),
            })?;

    let client = params.client.clone();
    let backend = params.backend.clone();
    let block_announce_validator = build_block_announce_validator(
        relay_chain_full_node.client.clone(),
        id,
        Box::new(relay_chain_full_node.network.clone()),
        relay_chain_full_node.backend.clone(),
    );

    let prometheus_registry = parachain_config.prometheus_registry().cloned();
    let force_authoring = parachain_config.force_authoring;
    let is_authority = parachain_config.role.is_authority();
    let transaction_pool = params.transaction_pool.clone();
    let mut task_manager = params.task_manager;
    let import_queue = cumulus_client_service::SharedImportQueue::new(params.import_queue);
    let (network, system_rpc_tx, start_network) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &parachain_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue: import_queue.clone(),
            on_demand: None,
            block_announce_validator_builder: Some(Box::new(|_| block_announce_validator)),
            warp_sync: None,
        })?;

    let rpc_client = client.clone();
    let rpc_pool = transaction_pool.clone();
    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        on_demand: None,
        remote_blockchain: None,
        rpc_extensions_builder: Box::new(move |deny_unsafe, _| {
            let mut io = jsonrpc_core::IoHandler::default();
            io.extend_with(SystemApi::to_delegate(FullSystem::new(
                rpc_client.clone(),
                rpc_pool.clone(),
                deny_unsafe,
            )));
            Ok(io)
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

    if is_authority {
        let parachain_consensus = build_consensus(
            id,
            lighthouse_account,
            client.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|t| t.handle()),
            &task_manager,
            &relay_chain_full_node,
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
            relay_chain_full_node,
            spawner,
            parachain_consensus,
        };

        start_collator(params).await?;
    } else {
        let params = StartFullNodeParams {
            client: client.clone(),
            announce_block,
            task_manager: &mut task_manager,
            para_id: id,
            relay_chain_full_node,
        };

        start_full_node(params)?;
    }

    start_network.start_network();

    Ok(task_manager)
}

/// Build the import queue for the PoS parachain runtime.
pub fn build_pos_import_queue<RuntimeApi, Executor>(
    client: Arc<TFullClient<Block, RuntimeApi, Executor>>,
    config: &sc_service::Configuration,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> Result<
    sc_consensus::DefaultImportQueue<Block, TFullClient<Block, RuntimeApi, Executor>>,
    sc_service::Error,
>
where
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    RuntimeApi: sp_api::ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, Executor>>
        + Send
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
        + AuraApi<Block, AuraId>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
{
    let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

    cumulus_client_consensus_aura::import_queue::<
        sp_consensus_aura::sr25519::AuthorityPair,
        _,
        _,
        _,
        _,
        _,
        _,
    >(cumulus_client_consensus_aura::ImportQueueParams {
        block_import: client.clone(),
        client: client.clone(),
        create_inherent_data_providers: move |_, _| async move {
            let time = sp_timestamp::InherentDataProvider::from_system_time();

            let slot =
                sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
                    *time,
                    slot_duration.slot_duration(),
                );

            Ok((time, slot))
        },
        registry: config.prometheus_registry().clone(),
        can_author_with: sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
        spawner: &task_manager.spawn_essential_handle(),
        telemetry,
    })
    .map_err(Into::into)
}

/// Build the import queue for the open consensus parachain runtime.
pub fn build_open_import_queue<RuntimeApi, Executor>(
    client: Arc<TFullClient<Block, RuntimeApi, Executor>>,
    config: &sc_service::Configuration,
    _telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> Result<
    sc_consensus::DefaultImportQueue<Block, TFullClient<Block, RuntimeApi, Executor>>,
    sc_service::Error,
>
where
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    RuntimeApi: sp_api::ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, Executor>>
        + Send
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
    para_id: polkadot_primitives::v0::Id,
    lighthouse_account: Option<AccountId>,
    client: Arc<TFullClient<Block, RuntimeApi, Executor>>,
    prometheus_registry: Option<&Registry>,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
    relay_chain_node: &polkadot_service::NewFull<polkadot_service::Client>,
    transaction_pool: Arc<
        sc_transaction_pool::FullPool<Block, TFullClient<Block, RuntimeApi, Executor>>,
    >,
    _sync_oracle: Arc<NetworkService<Block, Hash>>,
    _keystore: SyncCryptoStorePtr,
    _force_authoring: bool,
) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>
where
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    RuntimeApi: sp_api::ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, Executor>>
        + Send
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
    let account = lighthouse_account.unwrap_or(Default::default());
    let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
        task_manager.spawn_handle(),
        client.clone(),
        transaction_pool,
        prometheus_registry,
        telemetry.clone(),
    );
    let relay_chain_backend = relay_chain_node.backend.clone();
    let relay_chain_client = relay_chain_node.client.clone();

    let consensus = build_relay_chain_consensus(BuildRelayChainConsensusParams {
        para_id,
        proposer_factory,
        block_import: client.clone(),
        relay_chain_client: relay_chain_node.client.clone(),
        relay_chain_backend: relay_chain_node.backend.clone(),
        create_inherent_data_providers: move |_, (relay_parent, validation_data)| {
            let encoded_account = account.encode();
            let parachain_inherent = ParachainInherentData::create_at_with_client(
                relay_parent,
                &relay_chain_client,
                &*relay_chain_backend,
                &validation_data,
                para_id,
            );
            async move {
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

/// Build the PoS consensus.
pub fn build_pos_consensus<RuntimeApi, Executor>(
    para_id: polkadot_primitives::v0::Id,
    _lighthouse_account: Option<AccountId>,
    client: Arc<TFullClient<Block, RuntimeApi, Executor>>,
    prometheus_registry: Option<&Registry>,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
    relay_chain_node: &polkadot_service::NewFull<polkadot_service::Client>,
    transaction_pool: Arc<
        sc_transaction_pool::FullPool<Block, TFullClient<Block, RuntimeApi, Executor>>,
    >,
    sync_oracle: Arc<NetworkService<Block, Hash>>,
    keystore: SyncCryptoStorePtr,
    force_authoring: bool,
) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>
where
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    RuntimeApi: sp_api::ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, Executor>>
        + Send
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
        + AuraApi<Block, AuraId>,
    sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
{
    let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client).unwrap();

    let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
        task_manager.spawn_handle(),
        client.clone(),
        transaction_pool,
        prometheus_registry,
        telemetry.clone(),
    );

    let relay_chain_backend = relay_chain_node.backend.clone();
    let relay_chain_client = relay_chain_node.client.clone();
    let consensus = build_aura_consensus::<
        sp_consensus_aura::sr25519::AuthorityPair,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
        _,
    >(BuildAuraConsensusParams {
        proposer_factory,
        create_inherent_data_providers: move |_, (relay_parent, validation_data)| {
            let parachain_inherent =
                cumulus_primitives_parachain_inherent::ParachainInherentData::create_at_with_client(
                    relay_parent,
                    &relay_chain_client,
                    &*relay_chain_backend,
                    &validation_data,
                    para_id,
                );
            async move {
                let time = sp_timestamp::InherentDataProvider::from_system_time();
                let slot =
                    sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
                        *time,
                        slot_duration.slot_duration(),
                    );

                let parachain_inherent = parachain_inherent.ok_or_else(|| {
                    Box::<dyn std::error::Error + Send + Sync>::from(
                        "Failed to create parachain inherent",
                    )
                })?;
                Ok((time, slot, parachain_inherent))
            }
        },
        block_import: client.clone(),
        relay_chain_client: relay_chain_node.client.clone(),
        relay_chain_backend: relay_chain_node.backend.clone(),
        para_client: client.clone(),
        backoff_authoring_blocks: Option::<()>::None,
        sync_oracle,
        keystore,
        force_authoring,
        slot_duration,
        // We got around 500ms for proposing
        block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
        max_block_proposal_slot_portion: None,
        telemetry,
    });

    Ok(consensus)
}
