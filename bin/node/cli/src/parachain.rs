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

use codec::Encode;
use node_primitives::Block;
use robonomics_parachain_runtime::RuntimeApi;
use sc_service::{Configuration, PartialComponents, TFullBackend, TFullClient};
use sp_runtime::traits::{
    Block as BlockT, Hash as HashT, Header as HeaderT, Zero, BlakeTwo256,
};
use sp_trie::PrefixedMemoryDB;
use std::sync::Arc;

sc_executor::native_executor_instance!(
    pub Executor,
    robonomics_parachain_runtime::api::dispatch,
    robonomics_parachain_runtime::native_version,
);

pub fn new_partial(
    config: &mut Configuration,
) -> Result<
    PartialComponents<
        TFullClient<Block, RuntimeApi, Executor>,
        TFullBackend<Block>,
        (),
        sp_consensus::import_queue::BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
        sc_transaction_pool::FullPool<Block, TFullClient<Block, RuntimeApi, Executor>>,
        (),
    >,
    sc_service::Error,
> {
    let inherent_data_providers = sp_inherents::InherentDataProviders::new();

    let (client, backend, keystore, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
    let client = Arc::new(client);
    let registry = config.prometheus_registry();

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.prometheus_registry(),
        task_manager.spawn_handle(),
        client.clone(),
    );

    let import_queue = cumulus_consensus::import_queue::import_queue(
        client.clone(),
        client.clone(),
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        registry.clone(),
    )?;

    let params = PartialComponents {
        backend,
        client,
        import_queue,
        keystore,
        task_manager,
        transaction_pool,
        inherent_data_providers,
        select_chain: (),
        other: (),
    };

    Ok(params)
}

pub fn load_spec(
	id: &str,
	para_id: cumulus_primitives::ParaId,
) -> Result<Box<dyn sc_service::ChainSpec>, String> {
	match id {
		"" | "robonomics" => Ok(Box::new(chain_spec::ChainSpec::from_json_bytes(
			&include_bytes!("../res/robonomics_parachain.json")[..],
		)?)),
		"local_testnet" => Ok(Box::new(chain_spec::get_chain_spec(para_id))),
		path => Ok(Box::new(chain_spec::ChainSpec::from_json_file(
			path.into(),
		)?)),
	}
}

pub fn extract_genesis_wasm(
    chain_spec: &Box<dyn sc_service::ChainSpec>,
) -> sc_cli::Result<Vec<u8>> {
	let mut storage = chain_spec.build_storage()?;

	storage
		.top
		.remove(sp_core::storage::well_known_keys::CODE)
		.ok_or_else(|| "Could not find wasm file in genesis state!".into())
}

pub fn generate_genesis_state(
    chain_spec: &Box<dyn sc_service::ChainSpec>,
) -> sc_service::error::Result<Block> {
	let storage = chain_spec.build_storage()?;

    let child_roots = storage.children_default.iter().map(|(sk, child_content)| {
        let state_root = <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
            child_content.data.clone().into_iter().collect(),
        );
        (sk.clone(), state_root.encode())
    });
    let state_root = <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
        storage.top.clone().into_iter().chain(child_roots).collect(),
    );

    let extrinsics_root =
        <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(Vec::new());

    Ok(Block::new(
        <<Block as BlockT>::Header as HeaderT>::new(
            Zero::zero(),
            extrinsics_root,
            state_root,
            Default::default(),
            Default::default(),
        ),
        Default::default(),
    ))
}

pub mod chain_spec;
pub mod collator;
pub mod command;
