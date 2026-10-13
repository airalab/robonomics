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
//! # Robonomics Runtime Metadata
//!
//! This crate is the single source of truth for SCALE-encoded Robonomics
//! runtime metadata. It owns the build logic required to obtain, validate,
//! and expose the metadata as a reusable build artifact so that it can be
//! consumed by multiple downstream code generators (e.g. the Rust `subxt-api`
//! crate and future C++/embedded API generators) without those consumers
//! depending on each other.
//!
//! ## Build Modes
//!
//! - **Default**: the committed [`metadata.scale`](./metadata.scale) file is
//!   used as-is. This is the fastest option and doesn't require building the
//!   runtime.
//! - **`build-metadata`**: the Robonomics runtime WASM is built (via the
//!   `robonomics-runtime` build-dependency) and the `Metadata_metadata` host
//!   function is executed to produce fresh metadata.
//! - **`check-metadata`**: freshly generated metadata (as in `build-metadata`)
//!   is compared against the committed `metadata.scale`. The build fails if
//!   they differ, protecting against stale prebuilt metadata.
//!
//! ## Metadata Version
//!
//! The metadata is always V16, requested explicitly from the runtime by the
//! build script (the build fails if the runtime does not support it). V16
//! describes pallet view functions, which downstream generators rely on:
//! V14 and V15 carry none, so subxt would generate no typed view functions
//! from them. V16 is also the highest version both the Polkadot SDK
//! (`sp-metadata-ir`: V14-V16) and subxt 0.51 (V14-V16) support.
//!
//! ## Usage
//!
//! ```no_run
//! let metadata: &[u8] = robonomics_runtime_metadata::METADATA;
//! ```
//!
//! Consumers should not need to know how the metadata was obtained, nor
//! access this crate's `OUT_DIR` directly.

/// SCALE-encoded Robonomics runtime metadata used by consumers.
///
/// This always represents the metadata selected by this crate's active
/// feature configuration:
/// - The committed `metadata.scale` by default.
/// - Freshly extracted runtime metadata when the `build-metadata` (or
///   `check-metadata`) feature is enabled.
pub static METADATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/metadata.scale"));

#[cfg(test)]
mod tests {
    use super::METADATA;
    use frame_metadata::{RuntimeMetadata, RuntimeMetadataPrefixed};
    use parity_scale_codec::Decode;

    fn decode_v16() -> frame_metadata::v16::RuntimeMetadataV16 {
        match RuntimeMetadataPrefixed::decode(&mut &METADATA[..])
            .expect("metadata decodes")
            .1
        {
            RuntimeMetadata::V16(metadata) => metadata,
            other => panic!("expected metadata V16, got V{}", other.version()),
        }
    }

    #[test]
    fn metadata_is_v16() {
        assert_eq!(METADATA[4], 16);
        decode_v16();
    }

    #[test]
    fn metadata_describes_cps_view_functions() {
        let metadata = decode_v16();
        let cps = metadata
            .pallets
            .iter()
            .find(|pallet| pallet.name == "CPS")
            .expect("CPS pallet is present");
        let names: Vec<&str> = cps
            .view_functions
            .iter()
            .map(|view_function| view_function.name.as_str())
            .collect();

        assert!(names.contains(&"resolve_scope"), "{names:?}");
        assert!(names.contains(&"has_capability"), "{names:?}");
        assert!(names.contains(&"identity_of"), "{names:?}");
        assert!(names.contains(&"node_by_identity"), "{names:?}");
        for name in [
            "parent",
            "children",
            "meta",
            "payload",
            "scope_of",
            "address_capability",
        ] {
            assert!(names.contains(&name), "{name} missing: {names:?}");
        }
    }

    #[test]
    fn metadata_exposes_generic_view_function_api_only() {
        let metadata = decode_v16();
        let apis: Vec<&str> = metadata.apis.iter().map(|api| api.name.as_str()).collect();

        assert!(apis.contains(&"RuntimeViewFunction"), "{apis:?}");
        assert!(!apis.contains(&"CpsApi"), "{apis:?}");
    }
}
