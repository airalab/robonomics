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
//! Build script for extracting runtime metadata.
//!
//! This script extracts metadata from the Robonomics runtime by executing the
//! `Metadata_metadata` host function in the runtime WASM. The extracted metadata
//! is saved to a file that the subxt macro reads at compile time to generate
//! type-safe APIs.
//!
//! ## Process Overview
//!
//! 1. **Load WASM**: Get the runtime WASM binary from the `robonomics-runtime` crate
//! 2. **Decompress**: Handle potentially compressed WASM blobs
//! 3. **Execute**: Run the metadata extraction function in a WASM executor
//! 4. **Decode**: Parse the SCALE-encoded metadata
//! 5. **Validate**: Check the metadata magic bytes to ensure validity
//! 6. **Save**: Write to `$OUT_DIR/metadata.scale` for the subxt macro
//!
//! ## Why This Approach?
//!
//! Traditional approaches embed the runtime WASM directly in the subxt macro,
//! which pulls in all runtime dependencies (hundreds of crates). This approach
//! extracts just the metadata, significantly reducing dependencies and build time.
//!
//! ## Rebuild Triggers
//!
//! This script triggers a rebuild when:
//! - The runtime source code changes (`../src`)
//! - The runtime Cargo.toml changes (`../Cargo.toml`)
//!
//! ## Error Handling
//!
//! The script will panic if:
//! - WASM_BINARY is not available (runtime didn't build correctly)
//! - WASM blob is invalid or corrupted
//! - Metadata extraction fails (runtime execution error)
//! - Metadata decoding fails (invalid SCALE encoding)
//! - Magic sequence is invalid (warns but continues)
//! - File I/O fails (can't write metadata file)

use parity_scale_codec::Decode;
use sc_executor::{WasmExecutionMethod, WasmExecutor};
use sc_executor_common::runtime_blob::RuntimeBlob;
use sp_maybe_compressed_blob::{decompress, CODE_BLOB_BOMB_LIMIT};
use std::{env, fs, path::PathBuf};

/// Metadata magic number: `[0x6d, 0x65, 0x74, 0x61]` (spells "meta" in ASCII)
///
/// This is prepended to all Substrate metadata to identify it as valid metadata.
/// The magic sequence helps detect corrupted or invalid metadata early.
pub type ReservedMeta = [u8; 4];
pub const META: ReservedMeta = [0x6d, 0x65, 0x74, 0x61]; // 1635018093 in decimal, "meta" as ASCII

fn main() {
    // The way to get metadata is to call the runtime's `Metadata_metadata` host function.
    // This approach is inspired by subwasm (https://github.com/chevdor/subwasm).

    // Step 1: Get Robonomics runtime WASM code from the runtime crate dependency.
    // The runtime must be built first for this to work.
    let wasm = robonomics_runtime::dev::WASM_BINARY.expect("WASM_BINARY is not available");

    // Step 2: Decompress the WASM if it's compressed.
    // Runtime WASM may be compressed to reduce size. We need the raw WASM for execution.
    let uncompressed_wasm = decompress(&wasm, CODE_BLOB_BOMB_LIMIT).expect("WASM blob is invalid");

    // Step 3: Create a RuntimeBlob from the uncompressed WASM.
    // This prepares the WASM for execution by parsing and validating it.
    let runtime_blob =
        RuntimeBlob::new(&uncompressed_wasm).expect("Unable to create RuntimeBlob from WASM");

    // Step 4: Set up the execution environment.
    // BasicExternalities provides minimal host functions needed for metadata extraction.
    let mut ext = sp_state_machine::BasicExternalities::default();

    // Step 5: Create a WASM executor with sensible defaults.
    // The executor runs the WASM code in a sandboxed environment.
    let executor: WasmExecutor<sp_io::SubstrateHostFunctions> = WasmExecutor::builder()
        .with_execution_method(WasmExecutionMethod::default())
        .with_offchain_heap_alloc_strategy(sc_executor::HeapAllocStrategy::Dynamic {
            maximum_pages: Some(64), // Limit heap to 64 pages (4MB)
        })
        .with_max_runtime_instances(8) // Allow up to 8 concurrent instances
        .with_runtime_cache_size(2) // Cache 2 runtime instances
        .build();

    // Step 6: Execute the Metadata_metadata function in the runtime.
    // This is a standard host function that all Substrate runtimes implement.
    // The result is SCALE-encoded metadata bytes.
    let metadata_encoded = executor
        .uncached_call(runtime_blob, &mut ext, true, "Metadata_metadata", &[])
        .expect("Unable to call Runtime");

    // Step 7: Decode the SCALE-encoded metadata.
    // The metadata is wrapped in a Vec<u8>, so we decode that first.
    let metadata =
        <Vec<u8>>::decode(&mut &metadata_encoded[..]).expect("Unable to decode metadata");

    // Step 8: Verify the metadata magic sequence.
    // This ensures the metadata is valid and hasn't been corrupted.
    // We warn instead of failing to allow debugging corrupted metadata.
    if metadata.len() < 4 || [metadata[0], metadata[1], metadata[2], metadata[3]] != META {
        println!("cargo:warning=Invalid metadata magic sequence! Metadata broken?");
    }

    // Step 9: Write the metadata to a file for the subxt macro to consume.
    // The file is placed in OUT_DIR, which is a temporary directory for build outputs.
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let metadata_path = PathBuf::from(&out_dir).join("metadata.scale");
    fs::write(&metadata_path, metadata).expect("Failed to write metadata");

    // Step 10: Set up rebuild triggers.
    // These tell cargo to re-run this build script when the runtime changes.
    println!("cargo:rerun-if-changed=../src");
    println!("cargo:rerun-if-changed=../Cargo.toml");
}
