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
//! Build script for making runtime metadata available to the `subxt` macro.
//!
//! Runtime metadata extraction, validation and consistency checking is owned
//! by the `robonomics-runtime-metadata` crate. This script simply copies the
//! metadata exported by that crate (`robonomics_runtime_metadata::METADATA`)
//! into this crate's own `$OUT_DIR`, where the `subxt::subxt` procedural
//! macro expects to find it via `runtime_metadata_path = "$OUT_DIR/metadata.scale"`.

use std::{env, fs, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set"));

    fs::write(
        out_dir.join("metadata.scale"),
        robonomics_runtime_metadata::METADATA,
    )
    .expect("unable to write runtime metadata");
}
