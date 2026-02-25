# robonomics-metadata

Shared utility crate for exporting Robonomics runtime metadata at build time.

## Overview

This crate provides a single function `export_metadata()` that extracts metadata from the Robonomics runtime WASM binary. The metadata is needed by tools like `subxt` to generate type-safe APIs for interacting with the blockchain.

## Usage

Add to your `Cargo.toml`:

```toml
[build-dependencies]
robonomics-metadata = { path = "../robonomics-metadata" }
```

Then in your `build.rs`:

```rust
use robonomics_metadata::export_metadata;
use std::env;
use std::path::PathBuf;
use std::fs;

fn main() {
    let metadata = export_metadata().expect("Failed to export metadata");
    
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let metadata_path = PathBuf::from(&out_dir).join("metadata.scale");
    fs::write(&metadata_path, metadata).expect("Failed to write metadata");
    
    // Trigger rebuild if runtime changes
    println!("cargo:rerun-if-changed=../../runtime/robonomics/src");
    println!("cargo:rerun-if-changed=../../runtime/robonomics/Cargo.toml");
}
```

## Architecture

The crate uses the same approach as `subwasm`:

1. Load the Robonomics runtime WASM binary from `robonomics-runtime`
2. Decompress the WASM if needed
3. Create a runtime blob
4. Instantiate a WASM executor
5. Call the `Metadata_metadata` runtime function
6. Decode and return the metadata

## Used By

- `libcps` - For CPS pallet interaction
- `robonet` - For integration testing

## License

Apache 2.0
