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
//! # Robonomics Runtime Subxt API
//!
//! This crate provides a type-safe, compile-time verified API for interacting with the
//! Robonomics blockchain runtime. It extracts runtime metadata at build time and uses
//! [subxt](https://docs.rs/subxt) to generate type-safe transaction and storage APIs.
//!
//! ## Quick Start
//!
//! ```no_run
//! use robonomics_runtime_subxt_api::{api, RobonomicsConfig};
//! use subxt::OnlineClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to local node
//!     let client = OnlineClient::<RobonomicsConfig>::from_url("ws://127.0.0.1:9988").await?;
//!     
//!     // Query storage
//!     let block_number = client.blocks().at_latest().await?.number();
//!     println!("Latest block: {}", block_number);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## How It Works
//!
//! The `build.rs` script extracts metadata from the runtime and saves it to the build directory:
//!
//! 1. **Load runtime WASM**: Gets `WASM_BINARY` from robonomics-runtime build dependency
//! 2. **Create RuntimeBlob**: Prepares the WASM for execution
//! 3. **Execute metadata call**: Uses `WasmExecutor` to call the `Metadata_metadata` host function
//! 4. **Decode and validate**: Decodes SCALE-encoded metadata and validates magic bytes
//! 5. **Save to file**: Writes metadata to `$OUT_DIR/metadata.scale`
//! 6. **Subxt macro**: Reads the metadata file at compile time to generate type-safe APIs
//!
//! ## Benefits
//!
//! - **Fewer dependencies**: No need to embed runtime WASM or pull in heavy runtime dependencies
//! - **Faster builds**: Metadata extraction happens once during build
//! - **Always in sync**: Metadata comes directly from runtime dependency version
//! - **Type safe**: Compile-time verification of all runtime calls
//! - **Self-contained**: Everything happens in the build process
//!
//! ## API Usage
//!
//! The generated API provides access to:
//! - **Transactions**: `api::tx().pallet_name().call_name(...)`
//! - **Storage**: `api::storage().pallet_name().storage_name(...)`
//! - **Constants**: `api::constants().pallet_name().constant_name()`
//! - **Events**: `api::pallet_name::events::EventName { ... }`
//!
//! ### Example: Submit Transaction
//!
//! ```no_run
//! # use robonomics_runtime_subxt_api::{api, RobonomicsConfig};
//! # use subxt::OnlineClient;
//! # use subxt_signer::sr25519::dev;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = OnlineClient::<RobonomicsConfig>::from_url("ws://127.0.0.1:9988").await?;
//! let alice = dev::alice();
//!
//! // Create and submit a transaction
//! let tx = api::tx().system().remark(vec![1, 2, 3, 4]);
//! let hash = client.tx().sign_and_submit_default(&tx, &alice).await?;
//! println!("Transaction hash: {:?}", hash);
//! # Ok(())
//! # }
//! ```
//!
//! ### Example: Query Storage
//!
//! ```no_run
//! # use robonomics_runtime_subxt_api::{api, RobonomicsConfig, AccountId32};
//! # use subxt::OnlineClient;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = OnlineClient::<RobonomicsConfig>::from_url("ws://127.0.0.1:9988").await?;
//! let alice: AccountId32 = subxt_signer::sr25519::dev::alice().public_key().into();
//!
//! // Query account information
//! let account = client
//!     .storage()
//!     .at_latest()
//!     .await?
//!     .fetch(&api::storage().system().account(&alice))
//!     .await?;
//! println!("Account: {:?}", account);
//! # Ok(())
//! # }
//! ```
//!
//! For more examples, see the [README](../README.md).
// Re-export types
pub use api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
pub use subxt::utils::{AccountId32, MultiAddress, MultiSignature};

use subxt::config::DefaultExtrinsicParams;
use subxt::config::DefaultExtrinsicParamsBuilder;
use subxt::SubstrateConfig;

/// Type for extrinsic events from blockchain transactions.
///
/// This type is used to represent events emitted by extrinsics in blocks.
/// It provides methods to iterate over events and filter by type.
///
/// # Example
///
/// ```no_run
/// # use robonomics_runtime_subxt_api::{api, RobonomicsConfig, ExtrinsicEvents};
/// # use subxt::OnlineClient;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let client = OnlineClient::<RobonomicsConfig>::from_url("ws://127.0.0.1:9988").await?;
/// let block = client.blocks().at_latest().await?;
/// let extrinsics = block.extrinsics().await?;
///
/// for ext in extrinsics.iter() {
///     let events = ext.events().await?;
///     for event in events.iter() {
///         let event = event?;
///         println!("Event: {}::{}", event.pallet_name(), event.variant_name());
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub type ExtrinsicEvents = subxt::blocks::ExtrinsicEvents<RobonomicsConfig>;

/// Default configuration type for Robonomics blockchain nodes.
///
/// This configuration defines all the type aliases and parameters needed to
/// interact with a Robonomics node. It uses standard Substrate types with
/// custom extrinsic parameters suited for Robonomics.
///
/// # Type Parameters
///
/// - **AccountId**: [`AccountId32`] - Standard 32-byte SS58 account identifier
/// - **Signature**: [`MultiSignature`] - Supports SR25519, ED25519, and ECDSA signatures
/// - **Hasher**: `BlakeTwo256` - Blake2b-256 hashing algorithm
/// - **Header**: Standard Substrate header with `u32` block numbers
/// - **AssetId**: `u32` - Asset identifier type for multi-asset support
/// - **Address**: [`MultiAddress`] - Address format supporting both account IDs and indices
/// - **ExtrinsicParams**: [`RobonomicsExtrinsicParams`] - Transaction parameters
///
/// # Example
///
/// ```no_run
/// use robonomics_runtime_subxt_api::RobonomicsConfig;
/// use subxt::OnlineClient;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create a client using RobonomicsConfig
/// let client = OnlineClient::<RobonomicsConfig>::from_url("ws://127.0.0.1:9988").await?;
/// # Ok(())
/// # }
/// ```
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
/// to construct a transaction for a Robonomics node.
///
/// This type alias uses the default Substrate extrinsic parameters, which include:
/// - **CheckNonZeroSender**: Ensures the sender is not the zero address
/// - **CheckSpecVersion**: Validates the runtime spec version
/// - **CheckTxVersion**: Validates the transaction version
/// - **CheckGenesis**: Validates the genesis hash
/// - **CheckMortality**: Handles transaction mortality (era)
/// - **CheckNonce**: Manages account nonce
/// - **CheckWeight**: Validates transaction weight
/// - **ChargeTransactionPayment**: Handles transaction fees
///
/// # Example
///
/// ```no_run
/// # use robonomics_runtime_subxt_api::{api, RobonomicsConfig, RobonomicsExtrinsicParams};
/// # use subxt::OnlineClient;
/// # use subxt_signer::sr25519::dev;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = OnlineClient::<RobonomicsConfig>::from_url("ws://127.0.0.1:9988").await?;
/// let alice = dev::alice();
/// let tx = api::tx().system().remark(vec![1, 2, 3]);
///
/// // Use default params
/// let hash = client.tx().sign_and_submit_default(&tx, &alice).await?;
/// # Ok(())
/// # }
/// ```
pub type RobonomicsExtrinsicParams<T> = DefaultExtrinsicParams<T>;

/// A builder which leads to [`RobonomicsExtrinsicParams`] being constructed.
///
/// This builder allows you to customize transaction parameters before submitting.
/// Use this when you need to set custom values for tips, mortality, nonce, etc.
///
/// # Example
///
/// ```no_run
/// # use robonomics_runtime_subxt_api::{api, RobonomicsConfig, RobonomicsExtrinsicParamsBuilder};
/// # use subxt::OnlineClient;
/// # use subxt::config::polkadot::PlainTip;
/// # use subxt_signer::sr25519::dev;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = OnlineClient::<RobonomicsConfig>::from_url("ws://127.0.0.1:9988").await?;
/// let alice = dev::alice();
/// let tx = api::tx().system().remark(vec![1, 2, 3]);
///
/// // Build custom params with a tip
/// let params = RobonomicsExtrinsicParamsBuilder::<RobonomicsConfig>::new()
///     .tip(PlainTip::new(1_000_000))
///     .build();
///
/// let hash = client.tx().sign_and_submit(&tx, &alice, params).await?;
/// # Ok(())
/// # }
/// ```
pub type RobonomicsExtrinsicParamsBuilder<T> = DefaultExtrinsicParamsBuilder<T>;

/// Generated runtime metadata from subxt.
#[allow(
    dead_code,
    unused_imports,
    non_camel_case_types,
    unreachable_patterns,
    missing_docs
)]
#[allow(clippy::all)]
#[allow(rustdoc::broken_intra_doc_links)]
/// Robonomics runtime API generated from runtime metadata.
/// This ensures metadata is always in sync with the runtime.
#[subxt::subxt(
    runtime_metadata_path = "$OUT_DIR/metadata.scale",
    derive_for_type(path = "pallet_robonomics_cps::NodeId", derive = "Copy"),
    derive_for_all_types = "Eq, PartialEq, Clone, parity_scale_codec::Encode, parity_scale_codec::Decode"
)]
pub mod api {}

/// Helper implementations for CPS (Cyber-Physical Systems) pallet types.
///
/// This module provides convenient conversions and constructors for working with
/// CPS node data, particularly for creating plain and encrypted node data.
pub mod cps_impls {
    use super::api::runtime_types::pallet_robonomics_cps::{DefaultEncryptedData, NodeData};
    use super::BoundedVec;

    impl NodeData {
        /// Create an encrypted AEAD NodeData from raw encrypted bytes.
        ///
        /// This is typically used after encrypting data with XChaCha20-Poly1305
        /// or another AEAD cipher.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use robonomics_runtime_subxt_api::cps_impls::NodeData;
        ///
        /// // Assume we have encrypted bytes from an AEAD cipher
        /// let encrypted_bytes = vec![/* encrypted data */];
        /// let node_data = NodeData::aead_from(encrypted_bytes);
        /// ```
        pub fn aead_from(v: Vec<u8>) -> Self {
            NodeData::Encrypted(DefaultEncryptedData::Aead(BoundedVec(v)))
        }
    }

    impl From<Vec<u8>> for NodeData {
        /// Create plain NodeData from a byte vector.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use robonomics_runtime_subxt_api::cps_impls::NodeData;
        ///
        /// let data = vec![1, 2, 3, 4];
        /// let node_data = NodeData::from(data);
        /// ```
        fn from(v: Vec<u8>) -> Self {
            NodeData::Plain(BoundedVec(v))
        }
    }

    impl From<String> for NodeData {
        /// Create plain NodeData from a String.
        ///
        /// The string is converted to UTF-8 bytes.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use robonomics_runtime_subxt_api::cps_impls::NodeData;
        ///
        /// let node_data = NodeData::from("Hello, CPS!".to_string());
        /// ```
        fn from(s: String) -> Self {
            Self::from(s.into_bytes())
        }
    }

    impl From<&str> for NodeData {
        /// Create plain NodeData from a string slice.
        ///
        /// # Example
        ///
        /// ```no_run
        /// use robonomics_runtime_subxt_api::cps_impls::NodeData;
        ///
        /// let node_data = NodeData::from("Hello, CPS!");
        /// ```
        fn from(s: &str) -> Self {
            Self::from(s.as_bytes().to_vec())
        }
    }
}
