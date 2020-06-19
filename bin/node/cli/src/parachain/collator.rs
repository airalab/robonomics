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
use polkadot_primitives::parachain::{CollatorPair, Id as ParaId};
use sc_service::{config::Configuration, AbstractService};
use std::sync::Arc;

/// Desired Robonomics Parachain ID
pub const PARA_ID: ParaId = ParaId::new(1000);

/// Create collator for the parachain.
pub fn new_collator(
    parachain_config: Configuration,
    key: Arc<CollatorPair>,
    polkadot_config: polkadot_collator::Configuration,
) -> sc_service::error::Result<impl AbstractService> {
    let parachain_config = cumulus_collator::prepare_collator_config(parachain_config);

    let (builder, inherent_data_providers) = new_parachain!(
        parachain_config,
        robonomics_parachain_runtime::RuntimeApi,
        super::executor::Robonomics
    );

    let announce_validator = cumulus_network::DelayedBlockAnnounceValidator::new();
    let block_announce_validator = announce_validator.clone();
    let service = builder
        .with_block_announce_validator(|_client| Box::new(block_announce_validator))?
        .build_full()?;

    let registry = service.prometheus_registry();
    let proposer_factory = sc_basic_authorship::ProposerFactory::new(
        service.client(),
        service.transaction_pool(),
        registry.as_ref(),
    );

    let network = service.network();
    let announce_block = Arc::new(move |hash, data| network.announce_block(hash, data));

    let builder = cumulus_collator::CollatorBuilder::new(
        proposer_factory,
        inherent_data_providers,
        service.client(),
        PARA_ID,
        service.client(),
        announce_block,
        announce_validator,
    );

    let polkadot_future = polkadot_collator::start_collator(
        builder,
        PARA_ID,
        key,
        polkadot_config,
        Some("[RelayChain] ".to_string()),
    )
    .map(|_| ());
    service.spawn_essential_task("polkadot", polkadot_future);

    log::info!(target: "collator", "Run with parachain id: {:?}", PARA_ID);
    Ok(service)
}
