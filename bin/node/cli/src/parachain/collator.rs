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

use super::{new_partial, Executor, RuntimeApi};
use cumulus_client_consensus_relay_chain::{
    build_relay_chain_consensus, BuildRelayChainConsensusParams,
};
use cumulus_client_network::build_block_announce_validator;
use cumulus_client_service::{
    prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_parachain_inherent::ParachainInherentData;
use node_primitives::Block;
use sc_service::{Configuration, Role, TFullClient, TaskManager};
use std::sync::Arc;

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    id: polkadot_primitives::v0::Id,
    validator_account: Option<sp_core::H160>,
) -> sc_service::error::Result<(TaskManager, Arc<TFullClient<Block, RuntimeApi, Executor>>)> {
    if matches!(parachain_config.role, Role::Light) {
        return Err("Light client not supported!".into());
    }

    let parachain_config = prepare_node_config(parachain_config);

    let params = new_partial(&parachain_config)?;

    let (mut telemetry, telemetry_worker_handle) = params.other;
    let relay_chain_full_node = cumulus_client_service::build_polkadot_full_node(
        polkadot_config,
        telemetry_worker_handle,
    )
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
        })?;

    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        on_demand: None,
        remote_blockchain: None,
        rpc_extensions_builder: Box::new(|_, _| ()),
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

    if let Some(account) = validator_account {
        let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool,
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|x| x.handle()),
        );
        let relay_chain_backend = relay_chain_full_node.backend.clone();
        let relay_chain_client = relay_chain_full_node.client.clone();
        let parachain_consensus = build_relay_chain_consensus(BuildRelayChainConsensusParams {
            para_id: id,
            proposer_factory,
            block_import: client.clone(),
            relay_chain_client: relay_chain_full_node.client.clone(),
            relay_chain_backend: relay_chain_full_node.backend.clone(),
            create_inherent_data_providers: move |_, (relay_parent, validation_data)| {
                let parachain_inherent = ParachainInherentData::create_at_with_client(
                    relay_parent,
                    &relay_chain_client,
                    &*relay_chain_backend,
                    &validation_data,
                    id,
                );
                async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                    let lighthouse = pallet_robonomics_lighthouse::InherentDataProvider(Vec::from(
                        account.as_ref(),
                    ));
                    let parachain = parachain_inherent.ok_or_else(|| {
                        Box::<dyn std::error::Error + Send + Sync>::from(
                            "Failed to create parachain inherent",
                        )
                    })?;
                    Ok((timestamp, lighthouse, parachain))
                }
            },
        });

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

    Ok((task_manager, client))
}

/// Start a normal parachain node.
pub async fn start_node(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    id: polkadot_primitives::v0::Id,
    validator_account: Option<sp_core::H160>,
) -> sc_service::error::Result<(TaskManager, Arc<TFullClient<Block, RuntimeApi, Executor>>)> {
    start_node_impl(
        parachain_config,
        polkadot_config,
        id,
        validator_account,
    )
    .await
}
