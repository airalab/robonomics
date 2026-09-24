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
