///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2023 Robonomics Network <research@robonomics.network>
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
//! Robonomics family executors.

/// Local DevNet executor.
pub mod local {
    pub use local_runtime::RuntimeApi;

    pub struct Executor;
    impl sc_executor::NativeExecutionDispatch for Executor {
        type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

        fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
            local_runtime::api::dispatch(method, data)
        }

        fn native_version() -> sc_executor::NativeVersion {
            local_runtime::native_version()
        }
    }
}

/*
/// Robonomics AlphaNet parachain on Airalab relaychain.
pub mod alpha {
    pub use alpha_runtime::RuntimeApi;

    pub struct Executor;
    impl sc_executor::NativeExecutionDispatch for Executor {
        type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

        fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
            alpha_runtime::api::dispatch(method, data)
        }

        fn native_version() -> sc_executor::NativeVersion {
            alpha_runtime::native_version()
        }
    }
}

/// Robonomics MainNet parachain on Kusama.
pub mod main {
    pub use main_runtime::RuntimeApi;

    pub struct Executor;
    impl sc_executor::NativeExecutionDispatch for Executor {
        type ExtendHostFunctions = ();

        fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
            main_runtime::api::dispatch(method, data)
        }

        fn native_version() -> sc_executor::NativeVersion {
            main_runtime::native_version()
        }
    }
}

/// IPCI parachain on Kusama.
pub mod ipci {
    pub use ipci_runtime::RuntimeApi;

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
}
*/
