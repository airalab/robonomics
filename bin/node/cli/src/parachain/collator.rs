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

use cumulus_network::DelayedBlockAnnounceValidator;
use cumulus_service::{
    prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use polkadot_service::{AbstractClient, RuntimeApiCollection};
pub use sc_executor::NativeExecutor;
use sc_informant::OutputFormat;
use sc_service::{Configuration, Role, TaskManager};
use sp_consensus::SyncOracle;
use sp_core::crypto::Pair;
use sp_runtime::traits::{BlakeTwo256, Block as BlockT};
use std::sync::Arc;

/// Run a node with the given parachain `Configuration` and relay chain `Configuration`
///
/// This function blocks until done.
pub fn run_node(
    parachain_config: Configuration,
    parachain_id: ParaId,
    collator_key: Arc<CollatorPair>,
    mut polkadot_config: polkadot_collator::Configuration,
    validator: bool,
) -> sc_service::error::Result<TaskManager> {
    if matches!(parachain_config.role, Role::Light) {
        return Err("Light client not supported!".into());
    }

    let mut parachain_config = prepare_collator_config(parachain_config);

    parachain_config.informant_output_format = OutputFormat {
        enable_color: true,
        prefix: "[Parachain] ".to_string(),
    };
    polkadot_config.informant_output_format = OutputFormat {
        enable_color: true,
        prefix: "[Relaychain] ".to_string(),
    };

    let params = super::new_partial(&mut parachain_config)?;
    params
        .inherent_data_providers
        .register_provider(sp_timestamp::InherentDataProvider)
        .unwrap();

    let client = params.client.clone();
    let backend = params.backend.clone();
    let block_announce_validator = DelayedBlockAnnounceValidator::new();
    let block_announce_validator_builder = {
        let block_announce_validator = block_announce_validator.clone();
        move |_| Box::new(block_announce_validator) as Box<_>
    };

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
            block_announce_validator_builder: Some(Box::new(block_announce_validator_builder)),
            finality_proof_request_builder: None,
            finality_proof_provider: None,
        })?;

    let rpc_extensions_builder = Box::new(|_| jsonrpc_core::IoHandler::default());
    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        on_demand: None,
        remote_blockchain: None,
        rpc_extensions_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        telemetry_connection_sinks: Default::default(),
        config: parachain_config,
        keystore: params.keystore,
        backend,
        network: network.clone(),
        network_status_sinks,
        system_rpc_tx,
    })?;

    let announce_block = Arc::new(move |hash, data| network.announce_block(hash, data));

    if validator {
        let proposer_factory = sc_basic_authorship::ProposerFactory::new(
            client.clone(),
            transaction_pool,
            prometheus_registry.as_ref(),
        );
        let params = StartCollatorParams {
            para_id: parachain_id,
            block_import: client.clone(),
            proposer_factory,
            inherent_data_providers: params.inherent_data_providers,
            block_status: client.clone(),
            announce_block,
            client: client.clone(),
            block_announce_validator,
            task_manager: &mut task_manager,
            polkadot_config,
            collator_key,
        };
        start_collator(params)?;
    } else {
        let params = StartFullNodeParams {
            client: client.clone(),
            announce_block,
            polkadot_config,
            collator_key,
            block_announce_validator,
            task_manager: &mut task_manager,
            para_id: parachain_id,
        };

        start_full_node(params)?;
    }

    start_network.start_network();

    Ok(task_manager)
}
