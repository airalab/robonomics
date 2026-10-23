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
//! `Metadata_metadata_versions` and `Metadata_metadata_at_version` runtime APIs
//! in the runtime WASM. The metadata version is requested explicitly
//! ([`METADATA_VERSION`]) so that the committed artifact stays deterministic
//! across SDK upgrades. The extracted metadata
//! is saved to a file that this crate re-exports as [`METADATA`](crate::METADATA)
//! for downstream consumers (e.g. the `subxt-api` crate or future embedded/C++
//! code generators) to use at compile time.
//!
//! ## Process Overview
//!
//! 1. **Load WASM**: Get the runtime WASM binary from the `robonomics-runtime` crate
//! 2. **Decompress**: Handle potentially compressed WASM blobs
//! 3. **Check support**: Verify the runtime supports [`METADATA_VERSION`]
//! 4. **Execute**: Request exactly that metadata version in a WASM executor
//! 5. **Decode**: Parse the SCALE-encoded metadata
//! 6. **Validate**: Check the metadata magic bytes and version
//! 7. **Save**: Write to `$OUT_DIR/metadata.scale` for [`METADATA`](crate::METADATA)
//!
//! ## Why This Approach?
//!
//! Traditional approaches embed the runtime WASM directly in downstream code
//! generators, which pulls in all runtime dependencies (hundreds of crates).
//! This approach extracts just the metadata, significantly reducing
//! dependencies and build time for every consumer.
//!
//! ## Error Handling
//!
//! The script will panic if:
//! - WASM_BINARY is not available (runtime didn't build correctly)
//! - WASM blob is invalid or corrupted
//! - Metadata extraction fails (runtime execution error)
//! - The runtime does not support [`METADATA_VERSION`]
//! - Metadata decoding fails (invalid SCALE encoding)
//! - Magic sequence or version byte is invalid
//! - File I/O fails (can't write metadata file)

use std::{env, fs, path::PathBuf};

/// Metadata magic number: `[0x6d, 0x65, 0x74, 0x61]` (spells "meta" in ASCII)
///
/// This is prepended to all Substrate metadata to identify it as valid metadata.
/// The magic sequence helps detect corrupted or invalid metadata early.
pub type ReservedMeta = [u8; 4];
pub const META: ReservedMeta = [0x6d, 0x65, 0x74, 0x61]; // 1635018093 in decimal, "meta" as ASCII

/// Metadata version published by the Robonomics runtime.
///
/// Version 16 is the first (and, in the used Polkadot SDK, the latest stable)
/// metadata version that describes pallet view functions: subxt fills the
/// view functions of V14/V15 metadata with an empty set, so typed view-function
/// access is only generated from V16. subxt 0.51 decodes V14-V16 (and legacy
/// versions), so V16 is the highest version it can consume. The version is
/// requested explicitly instead of picking the highest one supported by the
/// runtime, which keeps the generated artifacts deterministic across SDK upgrades.
pub const METADATA_VERSION: u32 = 16;

#[cfg(feature = "check-metadata")]
fn check_metadata() {
    fn hash_file(file_path: PathBuf) -> u64 {
        use seahash::SeaHasher;
        use std::hash::Hasher;
        use std::io::{BufReader, Read};

        let mut hasher = SeaHasher::new();
        let file = fs::File::open(&file_path)
            .expect(format!("Unable to open file: {}", file_path.display()).as_str());
        let mut reader = BufReader::new(file);
        let mut buffer = [0; 8192]; // 8KB read chunk
        loop {
            let bytes_read = reader.read(&mut buffer).expect("File read error");
            if bytes_read == 0 {
                break; // End
            }
            hasher.write(&buffer[..bytes_read]);
        }
        hasher.finish()
    }

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let metadata_path = PathBuf::from(&out_dir).join("metadata.scale");
    let metadata_hash = hash_file(metadata_path);

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let saved_metadata_path = PathBuf::from(&manifest_dir).join("metadata.scale");
    let saved_metadata_hash = hash_file(saved_metadata_path);

    if metadata_hash != saved_metadata_hash {
        panic!(
            "Metadata hash mismatch: {} vs {}",
            metadata_hash, saved_metadata_hash
        );
    }
}

/// Build runtime and extract metadata into file.
#[cfg(feature = "build-metadata")]
fn main() {
    use parity_scale_codec::{Decode, Encode};
    use polkadot_sdk::sc_executor::{WasmExecutionMethod, WasmExecutor};
    use polkadot_sdk::sc_executor_common::runtime_blob::RuntimeBlob;
    use polkadot_sdk::sp_maybe_compressed_blob::{decompress, CODE_BLOB_BOMB_LIMIT};
    use polkadot_sdk::{sc_executor, sp_io, sp_state_machine};

    // The way to get metadata is to call the runtime's `Metadata` runtime API.
    // This approach is inspired by subwasm (https://github.com/chevdor/subwasm).

    // Step 1: Get Robonomics runtime WASM code from the runtime crate dependency.
    // The runtime must be built first for this to work.
    let wasm = robonomics_runtime::WASM_BINARY.expect("WASM_BINARY is not available");

    // Step 2: Decompress the WASM if it's compressed.
    // Runtime WASM may be compressed to reduce size. We need the raw WASM for execution.
    let uncompressed_wasm = decompress(&wasm, CODE_BLOB_BOMB_LIMIT).expect("WASM blob is invalid");

    // Step 3: Set up the execution environment.
    // BasicExternalities provides minimal host functions needed for metadata extraction.
    let mut ext = sp_state_machine::BasicExternalities::default();

    // Step 4: Create a WASM executor with sensible defaults.
    // The executor runs the WASM code in a sandboxed environment.
    let executor: WasmExecutor<sp_io::SubstrateHostFunctions> = WasmExecutor::builder()
        .with_execution_method(WasmExecutionMethod::default())
        .with_offchain_heap_alloc_strategy(sc_executor::HeapAllocStrategy::Dynamic {
            maximum_pages: Some(64), // Limit heap to 64 pages (4MB)
        })
        .with_max_runtime_instances(8) // Allow up to 8 concurrent instances
        .with_runtime_cache_size(2) // Cache 2 runtime instances
        .build();

    // Calls a runtime API function; every call needs its own RuntimeBlob
    // because the executor consumes it.
    let mut call = |method: &str, data: &[u8]| -> Vec<u8> {
        let runtime_blob =
            RuntimeBlob::new(&uncompressed_wasm).expect("Unable to create RuntimeBlob from WASM");
        executor
            .uncached_call(runtime_blob, &mut ext, true, method, data)
            .unwrap_or_else(|e| panic!("Unable to call {method}: {e}"))
    };

    // Step 5: Verify that the runtime supports the requested metadata version.
    // Fail clearly instead of silently falling back to another version.
    let versions = <Vec<u32>>::decode(&mut &call("Metadata_metadata_versions", &[])[..])
        .expect("Unable to decode supported metadata versions");
    if !versions.contains(&METADATA_VERSION) {
        panic!(
            "Runtime does not support metadata V{METADATA_VERSION}, supported versions: {versions:?}"
        );
    }

    // Step 6: Request exactly the expected metadata version.
    // The result is `Option<OpaqueMetadata>`, i.e. `Option<Vec<u8>>` on the wire.
    let metadata = <Option<Vec<u8>>>::decode(
        &mut &call("Metadata_metadata_at_version", &METADATA_VERSION.encode())[..],
    )
    .expect("Unable to decode metadata")
    .unwrap_or_else(|| panic!("Runtime returned no metadata for V{METADATA_VERSION}"));

    // Step 7: Verify the metadata magic sequence and version.
    // This ensures the metadata is valid and hasn't been corrupted.
    if metadata.len() < 5 || metadata[..4] != META {
        panic!("Invalid metadata magic sequence! Metadata broken?");
    }
    if u32::from(metadata[4]) != METADATA_VERSION {
        panic!(
            "Unexpected metadata version {}, expected {METADATA_VERSION}",
            metadata[4]
        );
    }

    // Step 8: Prepare the output location.
    // Step 9: Write the metadata to a file for the subxt macro to consume.
    // The file is placed in OUT_DIR, which is a temporary directory for build outputs.
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let metadata_path = PathBuf::from(&out_dir).join("metadata.scale");
    fs::write(&metadata_path, metadata).expect("Failed to write metadata");

    #[cfg(feature = "check-metadata")]
    check_metadata();
}

/// Just use already extracted metadata.
#[cfg(not(feature = "build-metadata"))]
fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let saved_metadata_path = PathBuf::from(&manifest_dir).join("metadata.scale");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let metadata_path = PathBuf::from(&out_dir).join("metadata.scale");
    fs::copy(saved_metadata_path, metadata_path).expect("Unable to copy saved metadata");
}
