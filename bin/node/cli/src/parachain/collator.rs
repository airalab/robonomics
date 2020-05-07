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

#![warn(unused_extern_crates)]

use std::sync::Arc;
use futures::prelude::*;
use sc_service::{AbstractService, config::Configuration};
use polkadot_primitives::parachain::{Id as ParaId, CollatorPair};
pub use sc_executor::NativeExecutionDispatch;

/// Desired Robonomics Parachain ID
pub const PARA_ID: ParaId = ParaId::new(1000);

#[cfg(feature = "frame-benchmarking")]
pub mod executor {
    sc_executor::native_executor_instance!(
        pub Robonomics,
        robonomics_parachain_runtime::api::dispatch,
        robonomics_parachain_runtime::native_version,
        frame_benchmarking::benchmarking::HostFunctions,
    );
}

#[cfg(not(feature = "frame-benchmarking"))]
pub mod executor {
    sc_executor::native_executor_instance!(
        pub Robonomics,
        robonomics_parachain_runtime::api::dispatch,
        robonomics_parachain_runtime::native_version,
    );
}

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
macro_rules! new_full_start {
    ($config:expr) => {{
        let inherent_data_providers = sp_inherents::InherentDataProviders::new();

        let builder = sc_service::ServiceBuilder::new_full::<
            node_primitives::Block,
            robonomics_parachain_runtime::RuntimeApi,
            executor::Robonomics,
        >($config)?
        .with_select_chain(|_config, backend| Ok(sc_consensus::LongestChain::new(backend.clone())))?
        .with_transaction_pool(|config, client, _fetcher, prometheus_registry| {
            let pool_api = Arc::new(sc_transaction_pool::FullChainApi::new(client.clone()));
            Ok(sc_transaction_pool::BasicPool::new(config, pool_api, prometheus_registry))
        })?
        .with_import_queue(|_, client, _, _| {
            let import_queue = cumulus_consensus::import_queue::import_queue(
                client.clone(),
                client,
                inherent_data_providers.clone(),
            )?;
            Ok(import_queue)
        })?;

        (builder, inherent_data_providers)
    }}
}

pub fn new_collator(
    parachain_config: Configuration,
    key: Arc<CollatorPair>,
    polkadot_config: polkadot_collator::Configuration,
) -> sc_service::error::Result<impl AbstractService> {
    let (builder, inherent_data_providers) = new_full_start!(parachain_config);
    inherent_data_providers
        .register_provider(sp_timestamp::InherentDataProvider)
        .unwrap();

    let service = builder
        .with_finality_proof_provider(|client, backend| {
            // GenesisAuthoritySetProvider is implemented for StorageAndProofProvider
            let provider = client as Arc<dyn sc_finality_grandpa::StorageAndProofProvider<_, _>>;
            Ok(Arc::new(sc_finality_grandpa::FinalityProofProvider::new(backend, provider)) as _)
        })?
        .build()?;

    let proposer_factory = sc_basic_authorship::ProposerFactory::new(
        service.client(),
        service.transaction_pool(),
    );

    let block_import = service.client();
    let client = service.client();
    let builder = cumulus_collator::CollatorBuilder::new(
        proposer_factory,
        inherent_data_providers,
        block_import,
        PARA_ID,
        client,
        Arc::new(|_,_| ()),
    );

    let polkadot_future = polkadot_collator::start_collator(
        builder,
        PARA_ID,
        key,
        polkadot_config,
    ).map(|_| ());
    service.spawn_essential_task("polkadot", polkadot_future);

    log::info!(target: "collator", "Run with parachain id: {:?}", PARA_ID);
    Ok(service)
}
