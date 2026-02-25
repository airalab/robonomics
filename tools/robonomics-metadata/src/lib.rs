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
//! # Robonomics Metadata Export
//!
//! This crate provides a shared utility for generating Robonomics runtime metadata
//! at build time. It extracts metadata from the compiled WASM runtime and makes it
//! available for use with subxt and other tools.
//!
//! ## Usage in build.rs
//!
//! ```no_run
//! use robonomics_metadata::export_metadata;
//! use std::env;
//! use std::path::PathBuf;
//!
//! fn main() {
//!     let metadata = export_metadata().expect("Failed to export metadata");
//!     
//!     let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
//!     let metadata_path = PathBuf::from(&out_dir).join("metadata.scale");
//!     std::fs::write(&metadata_path, metadata).expect("Failed to write metadata");
//!     
//!     // Trigger rebuild if runtime changes
//!     println!("cargo:rerun-if-changed=../../runtime/robonomics/src");
//!     println!("cargo:rerun-if-changed=../../runtime/robonomics/Cargo.toml");
//! }
//! ```

use parity_scale_codec::Decode;
use sc_executor::{WasmExecutionMethod, WasmExecutor};
use sc_executor_common::runtime_blob::RuntimeBlob;
use sp_maybe_compressed_blob::{decompress, CODE_BLOB_BOMB_LIMIT};

/// This is a "magic" number signaling that our bytearray is substrate metadata.
pub type ReservedMeta = [u8; 4];
pub const META: ReservedMeta = [0x6d, 0x65, 0x74, 0x61]; // 1635018093 in decimal, 'atem' as string...

/// Exports metadata from the Robonomics runtime WASM binary.
///
/// This function:
/// 1. Loads the Robonomics runtime WASM code
/// 2. Decompresses it if necessary
/// 3. Creates a runtime blob
/// 4. Instantiates a WASM executor
/// 5. Calls the `Metadata_metadata` runtime function
/// 6. Returns the encoded metadata
///
/// # Errors
///
/// Returns an error if:
/// - WASM binary is not available
/// - WASM decompression fails
/// - Runtime blob creation fails
/// - Metadata extraction fails
/// - Metadata decoding fails
/// - Metadata has invalid magic sequence
///
/// # Example
///
/// ```no_run
/// let metadata = robonomics_metadata::export_metadata().unwrap();
/// assert!(metadata.len() > 4);
/// ```
pub fn export_metadata() -> Result<Vec<u8>, String> {
    // The way to get metadata is call runtime `Metadata_metadata` host function.
    // Inspired by subwasm (https://github.com/chevdor/subwasm).

    // Get Robonomics runtime WASM code from runtime crate
    let wasm = robonomics_runtime::dev::WASM_BINARY
        .ok_or("WASM_BINARY is not available")?;

    // Create runtime blob from WASM code
    let uncompressed_wasm = decompress(&wasm, CODE_BLOB_BOMB_LIMIT)
        .map_err(|e| format!("WASM blob is invalid: {:?}", e))?;
    let runtime_blob = RuntimeBlob::new(&uncompressed_wasm)
        .map_err(|e| format!("Unable to create RuntimeBlob from WASM: {:?}", e))?;

    // Instantiate WASM executor
    let mut ext = sp_state_machine::BasicExternalities::default();
    let executor: WasmExecutor<sp_io::SubstrateHostFunctions> = WasmExecutor::builder()
        .with_execution_method(WasmExecutionMethod::default())
        .with_offchain_heap_alloc_strategy(sc_executor::HeapAllocStrategy::Dynamic {
            maximum_pages: Some(64),
        })
        .with_max_runtime_instances(8)
        .with_runtime_cache_size(2)
        .build();

    // Call host function and get runtime metadata
    let metadata_encoded = executor
        .uncached_call(runtime_blob, &mut ext, true, "Metadata_metadata", &[])
        .map_err(|e| format!("Unable to call Runtime: {:?}", e))?;

    let metadata = <Vec<u8>>::decode(&mut &metadata_encoded[..])
        .map_err(|e| format!("Unable to decode metadata: {:?}", e))?;

    // Verify metadata magic sequence
    if metadata.len() < 4 || [metadata[0], metadata[1], metadata[2], metadata[3]] != META {
        return Err("Invalid metadata magic sequence! Metadata broken?".to_string());
    }

    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_metadata() {
        let metadata = export_metadata().expect("Failed to export metadata");
        
        // Check that metadata has reasonable size
        assert!(metadata.len() > 1000, "Metadata seems too small");
        
        // Verify magic bytes
        assert_eq!(&metadata[0..4], &META, "Invalid metadata magic sequence");
    }
}
