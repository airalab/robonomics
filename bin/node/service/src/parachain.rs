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
//! Robonomics Node as a parachain collator.

pub use cumulus_client_service::genesis::generate_genesis_block;
pub mod chain_spec;
pub mod cli;
pub mod command;
pub mod service;

pub fn load_spec(
    id: &str,
    para_id: cumulus_primitives_core::ParaId,
) -> Result<Box<dyn sc_service::ChainSpec>, String> {
    match id {
        "" => {
            if para_id == chain_spec::KUSAMA_ID.into() {
                Ok(Box::new(chain_spec::get_main_chain_spec()))
            } else {
                Ok(Box::new(chain_spec::get_alpha_chain_spec(para_id)))
            }
        }
        // Load Alpha chain spec by default
        path => Ok(Box::new(chain_spec::AlphaChainSpec::from_json_file(
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

/// Robonomics AlphaNet on Airalab relaychain.
pub mod alpha {
    pub use alpha_runtime::RuntimeApi;
    use robonomics_primitives::AccountId;

    sc_executor::native_executor_instance!(
        pub Executor,
        alpha_runtime::api::dispatch,
        alpha_runtime::native_version,
    );

    /// Start a normal parachain node.
    pub async fn start_node(
        parachain_config: sc_service::Configuration,
        polkadot_config: sc_service::Configuration,
        para_id: cumulus_primitives_core::ParaId,
        lighthouse_account: Option<AccountId>,
    ) -> sc_service::error::Result<sc_service::TaskManager> {
        super::service::start_node_impl::<RuntimeApi, Executor>(
            parachain_config,
            polkadot_config,
            para_id,
            lighthouse_account,
        )
        .await
    }
}

/// Robonomics MainNet on Kusama.
#[cfg(feature = "kusama")]
pub mod main {
    pub use main_runtime::RuntimeApi;
    use robonomics_primitives::AccountId;

    sc_executor::native_executor_instance!(
        pub Executor,
        main_runtime::api::dispatch,
        main_runtime::native_version,
    );

    /// Start a normal parachain node.
    pub async fn start_node(
        parachain_config: sc_service::Configuration,
        polkadot_config: sc_service::Configuration,
        para_id: cumulus_primitives_core::ParaId,
        lighthouse_account: Option<AccountId>,
    ) -> sc_service::error::Result<sc_service::TaskManager> {
        super::service::start_node_impl::<RuntimeApi, Executor>(
            parachain_config,
            polkadot_config,
            para_id,
            lighthouse_account,
        )
        .await
    }
}
