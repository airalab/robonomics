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

use cumulus_collator::{prepare_collator_config, CollatorBuilder};
use cumulus_network::{DelayedBlockAnnounceValidator, JustifiedBlockAnnounceValidator};
use polkadot_primitives::v0::{Block as PBlock, CollatorPair, Id as ParaId};
use polkadot_service::{AbstractClient, RuntimeApiCollection};
pub use sc_executor::NativeExecutor;
use sc_informant::OutputFormat;
use sc_service::{Configuration, Role, TaskManager};
use sp_consensus::SyncOracle;
use sp_core::crypto::Pair;
use sp_runtime::traits::{BlakeTwo256, Block as BlockT};
use std::sync::Arc;

/// Create collator for the parachain.
pub fn new_collator(
    parachain_config: Configuration,
    parachain_id: ParaId,
    key: Arc<CollatorPair>,
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

    let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        on_demand: None,
        remote_blockchain: None,
        rpc_extensions_builder: Box::new(|_| ()),
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

    if validator {
        let proposer_factory = sc_basic_authorship::ProposerFactory::new(
            client.clone(),
            transaction_pool,
            prometheus_registry.as_ref(),
        );

        let block_import = client.clone();
        let announce_block = Arc::new(move |hash, data| network.announce_block(hash, data));
        let builder = CollatorBuilder::new(
            proposer_factory,
            params.inherent_data_providers,
            block_import,
            client.clone(),
            parachain_id,
            client.clone(),
            announce_block,
            block_announce_validator,
        );

        let (polkadot_future, polkadot_task_manager) =
            polkadot_collator::start_collator(builder, parachain_id, key, polkadot_config)?;

        task_manager
            .spawn_essential_handle()
            .spawn("polkadot-collator", polkadot_future);
        task_manager.add_child(polkadot_task_manager);
    } else {
        let is_light = matches!(polkadot_config.role, Role::Light);
        let (polkadot_task_manager, client, handles) = if is_light {
            Err("Light client not supported.".into())
        } else {
            polkadot_service::build_full(
                polkadot_config,
                Some((key.public(), parachain_id)),
                None,
                false,
                6000,
                None,
            )
        }?;
        let polkadot_network = handles
            .polkadot_network
            .expect("polkadot service is started; qed");
        client.execute_with(SetDelayedBlockAnnounceValidator {
            block_announce_validator,
            para_id: parachain_id,
            polkadot_sync_oracle: Box::new(polkadot_network),
        });

        task_manager.add_child(polkadot_task_manager);
    }

    start_network.start_network();

    Ok(task_manager)
}

struct SetDelayedBlockAnnounceValidator<B: BlockT> {
    block_announce_validator: DelayedBlockAnnounceValidator<B>,
    para_id: ParaId,
    polkadot_sync_oracle: Box<dyn SyncOracle + Send>,
}

impl<B: BlockT> polkadot_service::ExecuteWithClient for SetDelayedBlockAnnounceValidator<B> {
    type Output = ();

    fn execute_with_client<Client, Api, Backend>(self, client: Arc<Client>) -> Self::Output
    where
        <Api as sp_api::ApiExt<PBlock>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
        Backend: sc_client_api::Backend<PBlock>,
        Backend::State: sp_api::StateBackend<BlakeTwo256>,
        Api: RuntimeApiCollection<StateBackend = Backend::State>,
        Client: AbstractClient<PBlock, Backend, Api = Api> + 'static,
    {
        self.block_announce_validator
            .set(Box::new(JustifiedBlockAnnounceValidator::new(
                client,
                self.para_id,
                self.polkadot_sync_oracle,
            )));
    }
}
