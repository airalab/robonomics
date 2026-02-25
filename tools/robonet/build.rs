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

use robonomics_metadata::export_metadata;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Export metadata using shared utility
    let metadata = export_metadata().expect("Failed to export metadata");

    // Write the metadata to a file so subxt macro can use it
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let metadata_path = PathBuf::from(&out_dir).join("metadata.scale");
    fs::write(&metadata_path, metadata).expect("Failed to write metadata");

    // Trigger rebuild if runtime changes
    println!("cargo:rerun-if-changed=../../runtime/robonomics/src");
    println!("cargo:rerun-if-changed=../../runtime/robonomics/Cargo.toml");
}
