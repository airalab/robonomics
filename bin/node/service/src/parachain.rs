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
    Ok(match id {
        "" => {
            if para_id == chain_spec::KUSAMA_ID.into() {
                Box::new(chain_spec::get_main_chain_spec())
            } else if para_id == chain_spec::IPCI_ID.into() {
                Box::new(chain_spec::get_ipci_chain_spec())
            } else {
                Box::new(chain_spec::get_alpha_chain_spec(para_id))
            }
        }
        // Load Alpha chain spec by default
        path => Box::new(chain_spec::AlphaChainSpec::from_json_file(path.into())?),
    })
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

    pub struct AlphaExecutor;
    impl sc_executor::NativeExecutionDispatch for AlphaExecutor {
        type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

        fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
            alpha_runtime::api::dispatch(method, data)
        }

        fn native_version() -> sc_executor::NativeVersion {
            alpha_runtime::native_version()
        }
    }

    /// Start a normal parachain node.
    pub async fn start_node(
        parachain_config: sc_service::Configuration,
        polkadot_config: sc_service::Configuration,
        para_id: cumulus_primitives_core::ParaId,
        lighthouse_account: Option<AccountId>,
        heartbeat_interval: u64,
    ) -> sc_service::error::Result<sc_service::TaskManager> {
        super::service::start_node_impl::<RuntimeApi, AlphaExecutor, _, _>(
            parachain_config,
            polkadot_config,
            para_id,
            lighthouse_account,
            super::service::build_open_import_queue,
            super::service::build_open_consensus,
            heartbeat_interval,
        )
        .await
    }
}

/// Robonomics MainNet on Kusama.
#[cfg(feature = "kusama")]
pub mod main {
    pub use main_runtime::RuntimeApi;
    use robonomics_primitives::AccountId;

    pub struct MainExecutor;
    impl sc_executor::NativeExecutionDispatch for MainExecutor {
        type ExtendHostFunctions = ();

        fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
            main_runtime::api::dispatch(method, data)
        }

        fn native_version() -> sc_executor::NativeVersion {
            main_runtime::native_version()
        }
    }

    /// Start a normal parachain node.
    pub async fn start_node(
        parachain_config: sc_service::Configuration,
        polkadot_config: sc_service::Configuration,
        para_id: cumulus_primitives_core::ParaId,
        lighthouse_account: Option<AccountId>,
        heartbeat_interval: u64,
    ) -> sc_service::error::Result<sc_service::TaskManager> {
        super::service::start_node_impl::<RuntimeApi, MainExecutor, _, _>(
            parachain_config,
            polkadot_config,
            para_id,
            lighthouse_account,
            super::service::build_open_import_queue,
            super::service::build_open_consensus,
            heartbeat_interval,
        )
        .await
    }
}

/// IPCI Network parachain.
#[cfg(feature = "ipci")]
pub mod ipci {
    pub use ipci_runtime::RuntimeApi;
    use robonomics_primitives::AccountId;

    pub struct IPCIExecutor;
    impl sc_executor::NativeExecutionDispatch for IPCIExecutor {
        type ExtendHostFunctions = ();

        fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
            ipci_runtime::api::dispatch(method, data)
        }

        fn native_version() -> sc_executor::NativeVersion {
            ipci_runtime::native_version()
        }
    }

    /// Start a normal parachain node.
    pub async fn start_node(
        parachain_config: sc_service::Configuration,
        polkadot_config: sc_service::Configuration,
        para_id: cumulus_primitives_core::ParaId,
        lighthouse_account: Option<AccountId>,
        heartbeat_interval: u64,
    ) -> sc_service::error::Result<sc_service::TaskManager> {
        super::service::start_node_impl::<RuntimeApi, IPCIExecutor, _, _>(
            parachain_config,
            polkadot_config,
            para_id,
            lighthouse_account,
            super::service::build_pos_import_queue,
            super::service::build_pos_consensus,
            heartbeat_interval,
        )
        .await
    }
}
