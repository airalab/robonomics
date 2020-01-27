///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2019 Airalab <research@aira.life> 
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
//! A `CodeExecutor` specialization which uses natively compiled runtime when the wasm to be
//! executed is equivalent to the natively compiled code.

pub use sc_executor::NativeExecutor;
use sc_executor::native_executor_instance;

#[cfg(feature = "robonomics-runtime")]
native_executor_instance!(
    pub Executor,
    robonomics_runtime::api::dispatch,
    robonomics_runtime::native_version
);

#[cfg(feature = "robonomics-runtime")]
pub use robonomics_runtime as runtime;

#[cfg(feature = "ipci-runtime")]
native_executor_instance!(
    pub Executor,
    ipci_runtime::api::dispatch,
    ipci_runtime::native_version
);

#[cfg(feature = "ipci-runtime")]
pub use ipci_runtime as runtime;
