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

use futures::FutureExt;
use polkadot_primitives::v0::{CollatorPair, Id as ParaId};
use sc_informant::OutputFormat;
use sc_service::{Configuration, TaskManager};
use std::sync::Arc;

/// Create collator for the parachain.
pub fn new_collator(
    parachain_config: Configuration,
    parachain_id: ParaId,
    key: Arc<CollatorPair>,
    mut polkadot_config: polkadot_collator::Configuration,
) -> sc_service::error::Result<TaskManager> {
    let mut parachain_config = cumulus_collator::prepare_collator_config(parachain_config);
    parachain_config.informant_output_format = OutputFormat {
        enable_color: true,
        prefix: "[Parachain] ".to_string(),
    };

    let (params, inherent_data_providers, block_announce_validator) =
        super::new_parachain(parachain_config)?;

    let client = params.client.clone();
    let proposer_factory = sc_basic_authorship::ProposerFactory::new(
        client.clone(),
        params.transaction_pool.clone(),
        params.config.prometheus_registry(),
    );

    let sc_service::ServiceComponents {
        task_manager,
        network,
        ..
    } = sc_service::build(params)?;

    let block_import = client.clone();
    let announce_block = Arc::new(Box::new(move |hash, data| {
        network.announce_block(hash, data)
    }));
    let builder = cumulus_collator::CollatorBuilder::new(
        proposer_factory,
        inherent_data_providers,
        block_import,
        client.clone(),
        parachain_id,
        client.clone(),
        announce_block,
        block_announce_validator,
    );

    polkadot_config.informant_output_format = OutputFormat {
        enable_color: true,
        prefix: "[Relaychain] ".to_string(),
    };
    let (polkadot_future, polkadot_task_manager) =
        polkadot_collator::start_collator(builder, parachain_id, key, polkadot_config)?;
    let polkadot_future = polkadot_future.then(move |_| {
        let _ = polkadot_task_manager;
        futures::future::ready(())
    });

    task_manager
        .spawn_essential_handle()
        .spawn("polkadot", polkadot_future);

    Ok(task_manager)
}
