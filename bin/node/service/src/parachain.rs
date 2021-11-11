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

    #[derive(Clone)]
    pub struct AlphaExecutor;

    impl sc_executor::NativeExecutionDispatch for AlphaExecutor {
        /// Only enable the benchmarking host functions when we actually want to benchmark.
        #[cfg(feature = "runtime-benchmarks")]
        type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;
        /// Otherwise we only use the default Substrate host functions.
        #[cfg(not(feature = "runtime-benchmarks"))]
        type ExtendHostFunctions = ();

        fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
            alpha_runtime::api::dispatch(method, data)
        }

        fn native_version() -> sc_executor::NativeVersion {
            alpha_runtime::native_version()
        }
    }

    //------------------------------------------------------
    // use log::trace;
    // use sc_executor::with_externalities_safe;
    // use sc_executor_common::runtime_blob::RuntimeBlob;
    // use sc_executor_common::wasm_runtime::{InvokeMethod, WasmInstance, WasmModule};
    // use sp_core::traits::Externalities;
    // use sp_core::traits::RuntimeSpawn;
    // use sp_core::traits::RuntimeSpawnExt;
    // use sp_tasks::new_async_externalities;
    // use std::collections::HashMap;
    // use std::panic::AssertUnwindSafe;
    // use std::sync::{atomic::AtomicU64, atomic::Ordering, mpsc, Arc};
    //
    // /// Helper inner struct to implement `RuntimeSpawn` extension.
    // pub struct RuntimeInstanceSpawn {
    //     module: Arc<dyn WasmModule>,
    //     tasks: parking_lot::Mutex<HashMap<u64, mpsc::Receiver<Vec<u8>>>>,
    //     counter: AtomicU64,
    //     scheduler: Box<dyn sp_core::traits::SpawnNamed>,
    // }
    //
    // impl RuntimeSpawn for RuntimeInstanceSpawn {
    //     fn spawn_call(&self, dispatcher_ref: u32, func: u32, data: Vec<u8>) -> u64 {
    //         let new_handle = self.counter.fetch_add(1, Ordering::Relaxed);
    //
    //         let (sender, receiver) = mpsc::channel();
    //         self.tasks.lock().insert(new_handle, receiver);
    //
    //         let module = self.module.clone();
    //         let scheduler = self.scheduler.clone();
    //         self.scheduler.spawn(
    //             "executor-extra-runtime-instance",
    //             Box::pin(async move {
    //                 let module = AssertUnwindSafe(module);
    //
    //                 let async_ext = match new_async_externalities(scheduler.clone()) {
    //                     Ok(val) => val,
    //                     Err(e) => {
    //                         log::error!(
    //                             target: "executor",
    //                             "Failed to setup externalities for async context: {}",
    //                             e,
    //                         );
    //
    //                         // This will drop sender and receiver end will panic
    //                         return;
    //                     }
    //                 };
    //
    //                 let mut async_ext = match async_ext.with_runtime_spawn(Box::new(
    //                     RuntimeInstanceSpawn::new(module.clone(), scheduler),
    //                 )) {
    //                     Ok(val) => val,
    //                     Err(e) => {
    //                         log::error!(
    //                             target: "executor",
    //                             "Failed to setup runtime extension for async externalities: {}",
    //                             e,
    //                         );
    //
    //                         // This will drop sender and receiver end will panic
    //                         return;
    //                     }
    //                 };
    //
    //                 let result = with_externalities_safe(&mut async_ext, move || {
    //                     // FIXME: Should be refactored to shared "instance factory".
    //                     // Instantiating wasm here every time is suboptimal at the moment, shared
    //                     // pool of instances should be used.
    //                     //
    //                     // https://github.com/paritytech/substrate/issues/7354
    //                     let mut instance = module
    //                         .new_instance()
    //                         .expect("Failed to create new instance from module");
    //
    //                     instance
    //                         .call(
    //                             InvokeMethod::TableWithWrapper {
    //                                 dispatcher_ref,
    //                                 func,
    //                             },
    //                             &data[..],
    //                         )
    //                         .expect("Failed to invoke instance.")
    //                 });
    //
    //                 match result {
    //                     Ok(output) => {
    //                         let _ = sender.send(output);
    //                     }
    //                     Err(error) => {
    //                         // If execution is panicked, the `join` in the original runtime code will
    //                         // panic as well, since the sender is dropped without sending anything.
    //                         log::error!("Call error in spawned task: {:?}", error);
    //                     }
    //                 }
    //             }),
    //         );
    //
    //         new_handle
    //     }
    //
    //     fn join(&self, handle: u64) -> Vec<u8> {
    //         let receiver = self
    //             .tasks
    //             .lock()
    //             .remove(&handle)
    //             .expect("No task for the handle");
    //         let output = receiver
    //             .recv()
    //             .expect("Spawned task panicked for the handle");
    //         output
    //     }
    // }
    //
    // impl RuntimeInstanceSpawn {
    //     pub fn new(
    //         module: Arc<dyn WasmModule>,
    //         scheduler: Box<dyn sp_core::traits::SpawnNamed>,
    //     ) -> Self {
    //         Self {
    //             module,
    //             scheduler,
    //             counter: 0.into(),
    //             tasks: HashMap::new().into(),
    //         }
    //     }
    //
    //     fn with_externalities_and_module(
    //         module: Arc<dyn WasmModule>,
    //         mut ext: &mut dyn Externalities,
    //     ) -> Option<Self> {
    //         ext.extension::<sp_core::traits::TaskExecutorExt>()
    //             .map(move |task_ext| Self::new(module, task_ext.clone()))
    //     }
    // }
    //
    // /// Pre-registers the built-in extensions to the currently effective externalities.
    // ///
    // /// Meant to be called each time before calling into the runtime.
    // fn preregister_builtin_ext(module: Arc<dyn WasmModule>) {
    //     sp_externalities::with_externalities(move |mut ext| {
    //         if let Some(runtime_spawn) =
    //             RuntimeInstanceSpawn::with_externalities_and_module(module, ext)
    //         {
    //             if let Err(e) = ext.register_extension(RuntimeSpawnExt(Box::new(runtime_spawn))) {
    //                 trace!(
    //                     target: "executor",
    //                     "Failed to register `RuntimeSpawnExt` instance on externalities: {:?}",
    //                     e,
    //                 )
    //             }
    //         }
    //     });
    // }
    // impl sp_core::traits::ReadRuntimeVersion for AlphaExecutor {
    //     fn read_runtime_version(
    //         &self,
    //         wasm_code: &[u8],
    //         ext: &mut dyn Externalities,
    //     ) -> std::result::Result<Vec<u8>, String> {
    //         let runtime_blob = RuntimeBlob::uncompress_if_needed(&wasm_code)
    //             .map_err(|e| format!("Failed to create runtime blob: {:?}", e))?;
    //
    //         if let Some(version) = sc_executor::read_embedded_version(&runtime_blob)
    //             .map_err(|e| format!("Failed to read the static section: {:?}", e))
    //             .map(|v| v.map(|v| v.encode()))?
    //         {
    //             return Ok(version);
    //         }
    //
    //         // If the blob didn't have embedded runtime version section, we fallback to the legacy
    //         // way of fetching the version: i.e. instantiating the given instance and calling
    //         // `Core_version` on it.
    //
    //         self.uncached_call(
    //             runtime_blob,
    //             ext,
    //             // If a runtime upgrade introduces new host functions that are not provided by
    //             // the node, we should not fail at instantiation. Otherwise nodes that are
    //             // updated could run this successfully and it could lead to a storage root
    //             // mismatch when importing this block.
    //             true,
    //             "Core_version",
    //             &[],
    //         )
    //     }
    // }
    // impl sp_core::traits::CodeExecutor for AlphaExecutor {
    //     type Error = crate::Error;
    //
    //     fn call<
    //         R: codec::Decode + codec::Encode + PartialEq,
    //         NC: FnOnce() -> std::result::Result<R, Box<dyn std::error::Error + Send + Sync>>
    //             + std::panic::UnwindSafe,
    //     >(
    //         &self,
    //         ext: &mut dyn sc_executor::Externalities,
    //         runtime_code: &sp_core::traits::RuntimeCode,
    //         method: &str,
    //         data: &[u8],
    //         _use_native: bool,
    //         _native_call: Option<NC>,
    //     ) -> (sc_service::error::Result<sp_core::NativeOrEncoded<R>>, bool) {
    //         let result = self.with_instance(
    //             runtime_code,
    //             ext,
    //             false,
    //             |module, mut instance, _onchain_version, mut ext| {
    //                 sc_executor::with_externalities_safe(&mut **ext, move || {
    //                     preregister_builtin_ext(module.clone());
    //                     instance
    //                         .call_export(method, data)
    //                         .map(sp_core::NativeOrEncoded::Encoded)
    //                 })
    //             },
    //         );
    //         (result, false)
    //     }
    // }
    // impl sc_executor::RuntimeVersionOf for AlphaExecutor {
    //     fn runtime_version(
    //         &self,
    //         ext: &mut dyn sc_executor::Externalities,
    //         runtime_code: &sp_core::traits::RuntimeCode,
    //     ) -> sc_executor::error::Result<sp_version::RuntimeVersion> {
    //         self.with_instance(
    //             runtime_code,
    //             ext,
    //             false,
    //             |_module, _instance, version, _ext| {
    //                 Ok(version
    //                     .cloned()
    //                     .ok_or_else(|| crate::Error::ApiError("Unknown version".into())))
    //             },
    //         )
    //     }
    // }
    //------------------------------------------------------

    /// Start a normal parachain node.
    pub async fn start_node(
        parachain_config: sc_service::Configuration,
        polkadot_config: sc_service::Configuration,
        para_id: cumulus_primitives_core::ParaId,
        lighthouse_account: Option<AccountId>,
        heartbeat_interval: u64,
    ) -> sc_service::error::Result<sc_service::TaskManager> {
        super::service::start_node_impl::<
            RuntimeApi,
            sc_executor::NativeElseWasmExecutor<AlphaExecutor>,
            _,
            _,
        >(
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

    #[derive(Clone)]
    pub struct Executor;

    impl sc_executor::NativeExecutionDispatch for Executor {
        type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

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
        super::service::start_node_impl::<RuntimeApi, Executor, _, _>(
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

    #[derive(Clone)]
    pub struct Executor;

    impl sc_executor::NativeExecutionDispatch for Executor {
        type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

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
        super::service::start_node_impl::<RuntimeApi, Executor, _, _>(
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
