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
//! Blockchain client and connection management.

pub mod client;

pub use client::{Client, Config};

// Re-export event types for CLI usage
pub use robonomics::runtime_types::bounded_collections::bounded_vec::BoundedVec;
pub use subxt::utils::{AccountId32, MultiAddress, MultiSignature};

use subxt::config::DefaultExtrinsicParams;
use subxt::config::DefaultExtrinsicParamsBuilder;
use subxt::SubstrateConfig;

/// Default set of commonly used types by Robonomics nodes.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum RobonomicsConfig {}

impl subxt::Config for RobonomicsConfig {
    type AccountId = <SubstrateConfig as subxt::Config>::AccountId;
    type Signature = <SubstrateConfig as subxt::Config>::Signature;
    type Hasher = <SubstrateConfig as subxt::Config>::Hasher;
    type Header = <SubstrateConfig as subxt::Config>::Header;
    type AssetId = <SubstrateConfig as subxt::Config>::AssetId;
    type Address = MultiAddress<Self::AccountId, ()>;
    type ExtrinsicParams = RobonomicsExtrinsicParams<Self>;
}

/// A struct representing the signed extra and additional parameters required
/// to construct a transaction for a polkadot node.
pub type RobonomicsExtrinsicParams<T> = DefaultExtrinsicParams<T>;

/// A builder which leads to [`RobonomicsExtrinsicParams`] being constructed.
/// This is what you provide to methods like `sign_and_submit()`.
pub type RobonomicsExtrinsicParamsBuilder<T> = DefaultExtrinsicParamsBuilder<T>;

// Generated runtime metadata from subxt
#[allow(
    dead_code,
    unused_imports,
    non_camel_case_types,
    unreachable_patterns,
    missing_docs
)]
#[allow(clippy::all)]
#[allow(rustdoc::broken_intra_doc_links)]
// Robonomics runtime API generated from runtime metadata.
// This ensures metadata is always in sync with the runtime.
#[subxt::subxt(
    runtime_metadata_path = "$OUT_DIR/metadata.scale",
    derive_for_type(path = "pallet_robonomics_cps::NodeId", derive = "Copy"),
    derive_for_all_types = "Eq, PartialEq, Clone, parity_scale_codec::Encode, parity_scale_codec::Decode"
)]
pub mod robonomics {}
