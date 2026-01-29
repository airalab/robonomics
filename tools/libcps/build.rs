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

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // The robonomics-runtime is built as a dependency.
    // Access the WASM binary that was built and included by substrate-wasm-builder.
    
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    
    // Get the WASM binary from the robonomics-runtime build
    // When substrate-wasm-builder runs, it generates the WASM and makes it available
    let wasm_binary = robonomics_runtime::WASM_BINARY
        .expect("Runtime WASM binary not available. Make sure runtime is built with std feature.");
    
    println!("cargo:warning=Using embedded runtime WASM ({} bytes)", wasm_binary.len());
    
    // Write the WASM to a file so subxt macro can use it
    let wasm_path = PathBuf::from(&manifest_dir).join("robonomics_runtime.compact.wasm");
    fs::write(&wasm_path, wasm_binary)
        .expect("Failed to write WASM file");
    
    // Extract metadata from the WASM binary
    // The WASM contains the metadata, we need to extract it in SCALE format
    let metadata = extract_metadata_from_wasm(wasm_binary)
        .expect("Failed to extract metadata from WASM");
    
    let metadata_path = PathBuf::from(&manifest_dir).join("metadata.scale");
    fs::write(&metadata_path, metadata)
        .expect("Failed to write metadata file");
    
    println!("cargo:warning=Metadata extracted successfully to {}", metadata_path.display());
    
    // Trigger rebuild if runtime changes
    println!("cargo:rerun-if-changed=../../runtime/robonomics/src");
    println!("cargo:rerun-if-changed=../../runtime/robonomics/Cargo.toml");
}

fn extract_metadata_from_wasm(wasm: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Parse the WASM module to find the metadata section
    // Substrate runtimes include metadata in a custom section called "metadata"
    
    // Look for the metadata in the WASM custom sections
    // The metadata is stored after the string "meta" followed by the SCALE-encoded metadata
    
    // Find the "meta" magic bytes (6D 65 74 61 in hex)
    let meta_magic = b"meta";
    
    for i in 0..wasm.len().saturating_sub(4) {
        if &wasm[i..i+4] == meta_magic {
            // Found the metadata marker
            // The metadata starts after these 4 bytes
            // It's SCALE-encoded RuntimeMetadataPrefixed
            
            // The format is: [meta][metadata_bytes]
            // We need to return everything after "meta"
            let metadata_start = i + 4;
            
            // Find the end by looking for the next section or end of relevant data
            // Substrate stores compact metadata, typically the rest after "meta" marker
            // Let's extract a reasonable chunk - metadata is usually < 500KB
            
            let potential_end = (metadata_start + 500_000).min(wasm.len());
            let metadata_candidate = &wasm[metadata_start..potential_end];
            
            // Verify it's valid SCALE-encoded metadata by checking the prefix
            // RuntimeMetadataPrefixed starts with "meta" magic (which we skip) 
            // followed by version byte
            if metadata_candidate.len() > 1 {
                // Try to decode to verify it's valid
                match subxt_metadata::Metadata::decode(metadata_candidate) {
                    Ok(metadata) => {
                        // Successfully decoded, now re-encode to get clean SCALE bytes
                        use parity_scale_codec::Encode;
                        return Ok(metadata.encode());
                    }
                    Err(_) => {
                        // This might not be the right metadata section, continue searching
                        continue;
                    }
                }
            }
        }
    }
    
    Err("Metadata not found in WASM binary".into())
}
