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

use wasm_builder_runner::{build_current_project_with_rustflags, WasmBuilderSource};

fn main() {
    build_current_project_with_rustflags(
        "wasm_binary.rs",
        WasmBuilderSource::Crates("1.0.6"),
        // This instructs LLD to export __heap_base as a global variable, which is used by the
        // external memory allocator.
        "-Clink-arg=--export=__heap_base",
    );
}
