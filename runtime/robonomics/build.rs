///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2026 Robonomics Network <research@robonomics.network>
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

#[cfg(all(not(feature = "metadata-hash"), feature = "std"))]
fn main() {
    if cfg!(feature = "dev-runtime") {
        substrate_wasm_builder::WasmBuilder::init_with_defaults()
            .enable_feature("dev-runtime")
            .build();
    } else {
        substrate_wasm_builder::WasmBuilder::build_using_defaults();
    }
}

#[cfg(all(feature = "metadata-hash", feature = "std"))]
fn main() {
    let builder =
        substrate_wasm_builder::WasmBuilder::init_with_defaults().enable_metadata_hash("XRT", 9);

    if cfg!(feature = "dev-runtime") {
        builder.enable_feature("dev-runtime").build();
    } else {
        builder.build();
    }
}

#[cfg(not(feature = "std"))]
fn main() {}
