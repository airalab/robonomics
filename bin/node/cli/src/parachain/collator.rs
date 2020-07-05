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
use polkadot_primitives::parachain::{self, CollatorPair};
use sc_informant::OutputFormat;
use sc_service::{config::Configuration, TaskManager};
use std::sync::Arc;

/// Create collator for the parachain.
pub fn new_collator(
    parachain_config: Configuration,
    parachain_id: parachain::Id,
    key: Arc<CollatorPair>,
    mut polkadot_config: polkadot_collator::Configuration,
) -> sc_service::error::Result<TaskManager> {
    let mut parachain_config = cumulus_collator::prepare_collator_config(parachain_config);
    parachain_config.informant_output_format = OutputFormat {
        enable_color: true,
        prefix: "[Parachain] ".to_string(),
    };

    let (builder, inherent_data_providers, announce_validator) = new_parachain!(
        parachain_config,
        robonomics_parachain_runtime::RuntimeApi,
        super::executor::Robonomics
    );

    inherent_data_providers
        .register_provider(sp_timestamp::InherentDataProvider)
        .unwrap();

    let service = builder.build_full()?;
    let proposer_factory = sc_basic_authorship::ProposerFactory::new(
        service.client.clone(),
        service.transaction_pool.clone(),
        service.prometheus_registry.as_ref(),
    );

    let block_import = service.client.clone();
    let client = service.client.clone();
    let network = service.network.clone();
    let announce_block = Arc::new(move |hash, data| network.announce_block(hash, data));
    let collator_builder = cumulus_collator::CollatorBuilder::new(
        proposer_factory,
        inherent_data_providers,
        block_import,
        client.clone(),
        parachain_id,
        client,
        announce_block,
        announce_validator,
    );

    polkadot_config.informant_output_format = OutputFormat {
        enable_color: true,
        prefix: "[Relaychain] ".to_string(),
    };
    let polkadot_future =
        polkadot_collator::start_collator(collator_builder, parachain_id, key, polkadot_config)
            .map(|_| ());
    service
        .task_manager
        .spawn_essential_handle()
        .spawn("polkadot", polkadot_future);

    Ok(service.task_manager)
}
