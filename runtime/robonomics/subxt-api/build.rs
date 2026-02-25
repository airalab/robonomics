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

use parity_scale_codec::Decode;
use sc_executor::{WasmExecutionMethod, WasmExecutor};
use sc_executor_common::runtime_blob::RuntimeBlob;
use sp_maybe_compressed_blob::{decompress, CODE_BLOB_BOMB_LIMIT};
use std::{env, fs, path::PathBuf};

/// This is a "magic" number signaling that our bytearray is substrate metadata.
pub type ReservedMeta = [u8; 4];
pub const META: ReservedMeta = [0x6d, 0x65, 0x74, 0x61]; // 1635018093 in decimal, 'atem' as string...

fn main() {
    // The way to get metadata is call runtime `Metadata_metadata` host function.
    // Inspired by subwasm (https://github.com/chevdor/subwasm).

    // Get Robonomics runtime WASM code from runtime crate
    let wasm = robonomics_runtime::dev::WASM_BINARY.expect("WASM_BINARY is not available");

    // Create runtime blob from WASM code
    let uncompressed_wasm = decompress(&wasm, CODE_BLOB_BOMB_LIMIT).expect("WASM blob is invalid");
    let runtime_blob =
        RuntimeBlob::new(&uncompressed_wasm).expect("Unable to create RuntimeBlob from WASM");

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
        .expect("Unable to call Runtime");

    let metadata =
        <Vec<u8>>::decode(&mut &metadata_encoded[..]).expect("Unable to decode metadata");

    // Verify metadata magic sequence
    if metadata.len() < 4 || [metadata[0], metadata[1], metadata[2], metadata[3]] != META {
        println!("cargo:warning=Invalid metadata magic sequence! Metadata broken?");
    }

    // Write the metadata to a file so subxt macro can use it
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let metadata_path = PathBuf::from(&out_dir).join("metadata.scale");
    fs::write(&metadata_path, metadata).expect("Failed to write metadata");

    // Trigger rebuild if runtime changes
    println!("cargo:rerun-if-changed=../src");
    println!("cargo:rerun-if-changed=../Cargo.toml");
}
