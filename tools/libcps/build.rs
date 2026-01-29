///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2025 Robonomics Network <research@robonomics.network>
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

use parity_scale_codec::Encode;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    
    // The robonomics-runtime is built as a dependency with std feature.
    // Access the metadata directly from the runtime
    let metadata = robonomics_runtime::RuntimeGenesisConfig::default()
        .build_storage()
        .expect("Failed to build storage")
        .top
        .into_iter()
        .find(|(k, _)| k == b":code")
        .map(|(_, v)| v)
        .expect("Runtime code not found in storage");
    
    // Actually, let's use the simpler approach: get metadata directly from the runtime API
    let metadata = <robonomics_runtime::Runtime as frame_metadata::RuntimeMetadata>::metadata();
    
    println!("cargo:warning=Extracted metadata from runtime");
    
    // Encode to SCALE format
    let metadata_bytes = metadata.encode();
    
    // Write the metadata to a file so subxt macro can use it
    let metadata_path = PathBuf::from(&manifest_dir).join("metadata.scale");
    fs::write(&metadata_path, metadata_bytes)
        .expect("Failed to write metadata file");
    
    println!("cargo:warning=Wrote metadata to {}", metadata_path.display());
    
    // Trigger rebuild if runtime changes
    println!("cargo:rerun-if-changed=../../runtime/robonomics/src");
    println!("cargo:rerun-if-changed=../../runtime/robonomics/Cargo.toml");
}
