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
//! Robonomics Node as a parachain collator.

use node_primitives::Block;
use robonomics_parachain_runtime::RuntimeApi;
use sc_service::{Configuration, Error as ServiceError};
use std::sync::Arc;

sc_executor::native_executor_instance!(
    pub Executor,
    robonomics_parachain_runtime::api::dispatch,
    robonomics_parachain_runtime::native_version,
);

type FullClient = sc_service::TFullClient<Block, RuntimeApi, Executor>;
type FullBackend = sc_service::TFullBackend<Block>;

pub fn new_parachain(
    config: Configuration,
) -> Result<
    (
        sc_service::ServiceParams<
            Block,
            FullClient,
            sc_consensus_babe::BabeImportQueue<Block, FullClient>,
            sc_transaction_pool::FullPool<Block, FullClient>,
            (),
            FullBackend,
        >,
        sp_inherents::InherentDataProviders,
        cumulus_network::DelayedBlockAnnounceValidator<Block>,
    ),
    ServiceError,
> {
    let inherent_data_providers = sp_inherents::InherentDataProviders::new();
    let (client, backend, keystore, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;

    let client = Arc::new(client);
    let pool_api =
        sc_transaction_pool::FullChainApi::new(client.clone(), config.prometheus_registry());
    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        Arc::new(pool_api),
        config.prometheus_registry(),
        task_manager.spawn_handle(),
        client.clone(),
    );

    let import_queue = cumulus_consensus::import_queue::import_queue(
        client.clone(),
        client.clone(),
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
    )?;

    inherent_data_providers
        .register_provider(sp_timestamp::InherentDataProvider)
        .unwrap();

    let block_announce_validator = cumulus_network::DelayedBlockAnnounceValidator::new();
    let bav_builder = {
        let validator = block_announce_validator.clone();
        move |_| Box::new(validator) as Box<_>
    };
    let params = sc_service::ServiceParams {
        config,
        client: client.clone(),
        backend,
        import_queue,
        keystore,
        task_manager,
        rpc_extensions_builder: Box::new(|_| ()),
        transaction_pool,
        block_announce_validator_builder: Some(Box::new(bav_builder)),
        finality_proof_request_builder: None,
        finality_proof_provider: None,
        on_demand: None,
        remote_blockchain: None,
    };

    Ok((params, inherent_data_providers, block_announce_validator))
}

pub mod chain_spec;
pub mod collator;
pub mod command;
