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
    // Find the workspace root
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = PathBuf::from(&manifest_dir);
    let workspace_root = manifest_path
        .parent()
        .and_then(|p| p.parent())
        .expect("Could not determine workspace root")
        .to_path_buf();
    
    let runtime_dir = workspace_root.join("runtime/robonomics");
    
    // Check for WASM file in release build
    let wasm_path = workspace_root
        .join("target/release/wbuild/robonomics-runtime/robonomics_runtime.compact.wasm");
    
    if !wasm_path.exists() {
        eprintln!();
        eprintln!("================================================================================");
        eprintln!("ERROR: Robonomics runtime WASM not found!");
        eprintln!();
        eprintln!("The WASM runtime must be built before building libcps.");
        eprintln!("Please run the following command first:");
        eprintln!();
        eprintln!("    cargo build -p robonomics-runtime --release");
        eprintln!();
        eprintln!("Expected WASM location:");
        eprintln!("    {}", wasm_path.display());
        eprintln!("================================================================================");
        eprintln!();
        panic!("Robonomics runtime WASM not found. Build the runtime first.");
    }
    
    println!("cargo:warning=Using WASM from: {}", wasm_path.display());
    
    // Copy WASM to a known location in the libcps directory
    let dest_wasm = manifest_path.join("robonomics_runtime.compact.wasm");
    
    fs::copy(&wasm_path, &dest_wasm)
        .expect("Failed to copy WASM to libcps directory");
    
    println!("cargo:warning=Copied WASM to: {}", dest_wasm.display());
    
    // Trigger rebuild if runtime source changes
    println!("cargo:rerun-if-changed={}", runtime_dir.join("src").display());
    println!("cargo:rerun-if-changed={}", runtime_dir.join("Cargo.toml").display());
    println!("cargo:rerun-if-changed={}", wasm_path.display());
}
