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
    // The robonomics-runtime build dependency will build the WASM runtime.
    // We need to find the path to the built WASM file and pass it to the compile-time
    // environment so the subxt macro can use it.
    
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = PathBuf::from(&out_dir);
    
    // Navigate from OUT_DIR to the target directory
    // OUT_DIR structure: target/<profile>/build/<crate-hash>/out
    let target_dir = out_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("Could not determine target directory");
    
    // Look for robonomics-runtime build directory
    let build_dir = target_dir.join("debug").join("build");
    
    let mut wasm_path = None;
    
    if let Ok(entries) = fs::read_dir(&build_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let dir_name = entry.file_name();
            if dir_name.to_string_lossy().starts_with("robonomics-runtime-") {
                let potential_wasm = entry
                    .path()
                    .join("out/wbuild/robonomics-runtime/robonomics_runtime.compact.wasm");
                if potential_wasm.exists() {
                    wasm_path = Some(potential_wasm);
                    break;
                }
            }
        }
    }
    
    let wasm_path = wasm_path.expect(
        "Could not find robonomics_runtime.compact.wasm. \
         Make sure robonomics-runtime is built as a dependency."
    );
    
    // Set the WASM path as an environment variable for use in the source code
    println!(
        "cargo:rustc-env=ROBONOMICS_RUNTIME_WASM={}",
        wasm_path.display()
    );
    
    // Make sure we rebuild if the WASM changes
    println!("cargo:rerun-if-changed={}", wasm_path.display());
    
    // Trigger rebuild if runtime source changes
    println!("cargo:rerun-if-changed=../../runtime/robonomics/src");
    println!("cargo:rerun-if-changed=../../runtime/robonomics/Cargo.toml");
}
