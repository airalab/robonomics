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
//! Polkadot collator service implementation.

use super::{new_partial, Executor, RuntimeApi};
use cumulus_network::build_block_announce_validator;
use cumulus_service::{
    prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use node_primitives::Block;
use polkadot_primitives::v0::CollatorPair;
use sc_service::{Configuration, Role, TFullClient, TaskManager};
use sp_core::Pair;
use std::sync::Arc;

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl(
    parachain_config: Configuration,
    collator_key: CollatorPair,
    polkadot_config: Configuration,
    id: polkadot_primitives::v0::Id,
    validator: bool,
) -> sc_service::error::Result<(TaskManager, Arc<TFullClient<Block, RuntimeApi, Executor>>)> {
    if matches!(parachain_config.role, Role::Light) {
        return Err("Light client not supported!".into());
    }

    let parachain_config = prepare_node_config(parachain_config);

    let polkadot_full_node =
        cumulus_service::build_polkadot_full_node(polkadot_config, collator_key.public()).map_err(
            |e| match e {
                polkadot_service::Error::Sub(x) => x,
                s => format!("{}", s).into(),
            },
        )?;

    let params = new_partial(&parachain_config)?;
    let telemetry_span = params.other;
    params
        .inherent_data_providers
        .register_provider(sp_timestamp::InherentDataProvider)
        .unwrap();

    let client = params.client.clone();
    let backend = params.backend.clone();
    let block_announce_validator = build_block_announce_validator(
        polkadot_full_node.client.clone(),
        id,
        Box::new(polkadot_full_node.network.clone()),
        polkadot_full_node.backend.clone(),
    );

    let prometheus_registry = parachain_config.prometheus_registry().cloned();
    let transaction_pool = params.transaction_pool.clone();
    let mut task_manager = params.task_manager;
    let import_queue = params.import_queue;
    let (network, network_status_sinks, system_rpc_tx, start_network) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &parachain_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: None,
            block_announce_validator_builder: Some(Box::new(|_| block_announce_validator)),
        })?;

    let rpc_extensions_builder = Box::new(|_, _| ());
    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        on_demand: None,
        remote_blockchain: None,
        rpc_extensions_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config: parachain_config,
        keystore: params.keystore_container.sync_keystore(),
        backend: backend.clone(),
        network: network.clone(),
        network_status_sinks,
        system_rpc_tx,
        telemetry_span,
    })?;

    let announce_block = {
        let network = network.clone();
        Arc::new(move |hash, data| network.announce_block(hash, Some(data)))
    };

    if validator {
        let proposer_factory = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool,
            prometheus_registry.as_ref(),
        );
        let spawner = task_manager.spawn_handle();
        let polkadot_backend = polkadot_full_node.backend.clone();

        let params = StartCollatorParams {
            para_id: id,
            block_import: client.clone(),
            proposer_factory,
            inherent_data_providers: params.inherent_data_providers,
            block_status: client.clone(),
            announce_block,
            client: client.clone(),
            task_manager: &mut task_manager,
            collator_key,
            polkadot_full_node,
            spawner,
            backend,
            polkadot_backend,
        };

        start_collator(params).await?;
    } else {
        let params = StartFullNodeParams {
            client: client.clone(),
            announce_block,
            task_manager: &mut task_manager,
            para_id: id,
            polkadot_full_node,
        };

        start_full_node(params)?;
    }

    start_network.start_network();

    Ok((task_manager, client))
}

/// Start a normal parachain node.
pub async fn start_node(
    parachain_config: Configuration,
    collator_key: CollatorPair,
    polkadot_config: Configuration,
    id: polkadot_primitives::v0::Id,
    validator: bool,
) -> sc_service::error::Result<(TaskManager, Arc<TFullClient<Block, RuntimeApi, Executor>>)> {
    start_node_impl(
        parachain_config,
        collator_key,
        polkadot_config,
        id,
        validator,
    )
    .await
}
