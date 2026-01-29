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
//! # libcps - Robonomics Cyber-Physical Systems Library
//!
//! `libcps` provides a comprehensive Rust library for interacting with the Robonomics
//! CPS (Cyber-Physical Systems) pallet. It enables developers to build applications
//! that manage hierarchical cyber-physical systems on the Robonomics blockchain with
//! support for encrypted data storage and IoT integration.
//!
//! ## Features
//!
//! - **Blockchain Integration**: Seamless interaction with Robonomics blockchain via subxt
//! - **Encryption**: XChaCha20-Poly1305 AEAD encryption with sr25519 key derivation
//! - **MQTT Bridge**: Bidirectional IoT device communication
//! - **Type Safety**: Strongly-typed APIs matching the CPS pallet
//! - **Async Support**: Built on tokio for efficient async operations
//!
//! ## Quick Start
//!
//! ```no_run
//! use libcps::{Client, Config};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Connect to blockchain
//!     let config = Config {
//!         ws_url: "ws://localhost:9944".to_string(),
//!         suri: Some("//Alice".to_string()),
//!     };
//!     
//!     let client = Client::new(&config).await?;
//!     
//!     // Use the client to interact with CPS pallet
//!     // (requires generated metadata from a running node)
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Modules
//!
//! - [`blockchain`]: Blockchain client and connection management
//! - [`crypto`]: Encryption and key derivation utilities
//! - [`mqtt`]: MQTT bridge configuration and types
//! - [`types`]: CPS pallet type definitions
//! - [`node`]: Node-oriented API with async/sync methods for CPS operations
//!
//! ## Encryption
//!
//! The library implements **AEAD encryption with multiple algorithms and schemes**:
//!
//! ```no_run
//! use libcps::crypto::{Cipher, EncryptionAlgorithm, CryptoScheme};
//!
//! # fn example() -> anyhow::Result<()> {
//! // Create a Cipher with SR25519 scheme
//! let sender_cipher = Cipher::new(
//!     "//Alice".to_string(),
//!     CryptoScheme::Sr25519,
//! )?;
//!
//! let receiver_cipher = Cipher::new(
//!     "//Bob".to_string(),
//!     CryptoScheme::Sr25519,
//! )?;
//!
//! let plaintext = b"secret message";
//! let receiver_public = receiver_cipher.public_key();
//!
//! // Encrypt using the cipher
//! let encrypted_msg = sender_cipher.encrypt(plaintext, &receiver_public, EncryptionAlgorithm::XChaCha20Poly1305)?;
//!
//! // Decrypt with optional sender verification
//! let sender_public = sender_cipher.public_key();
//! let decrypted = receiver_cipher.decrypt(&encrypted_msg, Some(&sender_public))?;
//! # Ok(())
//! # }
//! ```
//!
//! ## MQTT Bridge
//!
//! Configure and use MQTT bridge for IoT integration:
//!
//! ```no_run
//! use libcps::{mqtt, Config as BlockchainConfig};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Configure MQTT connection
//! let mqtt_config = mqtt::Config {
//!     broker: "mqtt://localhost:1883".to_string(),
//!     username: Some("user".to_string()),
//!     password: Some("pass".to_string()),
//!     client_id: Some("my-client".to_string()),
//!     blockchain: None,
//!     subscribe: Vec::new(),
//!     publish: Vec::new(),
//! };
//!
//! // Configure blockchain connection
//! let blockchain_config = BlockchainConfig {
//!     ws_url: "ws://localhost:9944".to_string(),
//!     suri: Some("//Alice".to_string()),
//! };
//!
//! // Subscribe to MQTT and update blockchain using Config method
//! mqtt_config.subscribe(
//!     &blockchain_config,
//!     None,              // No encryption
//!     "sensors/temp",    // MQTT topic
//!     1,                 // Node ID
//!     None,              // No receiver public key
//!     None,              // No algorithm
//!     None,              // No custom message handler
//! ).await?;
//!
//! // Or publish blockchain changes to MQTT using Config method
//! mqtt_config.publish(
//!     &blockchain_config,
//!     None,               // Optional cipher for decryption
//!     "actuators/status", // MQTT topic
//!     1,                  // Node ID
//!     None,               // No custom publish handler
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Configuration File Support
//!
//! Manage multiple bridges with a TOML configuration file:
//!
//! ```no_run
//! use libcps::mqtt::Config;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Load configuration from file
//! let config = Config::from_file("mqtt_config.toml")?;
//!
//! // Start all configured bridges concurrently
//! config.start().await?;
//! # Ok(())
//! # }
//! ```
//!
//! Example configuration file:
//! ```toml
//! broker = "mqtt://localhost:1883"
//!
//! [blockchain]
//! ws_url = "ws://localhost:9944"
//! suri = "//Alice"
//!
//! [[subscribe]]
//! topic = "sensors/temperature"
//! node_id = 5
//!
//! [[publish]]
//! topic = "actuators/valve"
//! node_id = 10
//! ```
//!
//! See [`mqtt`] module documentation and `examples/mqtt_config.toml` for more details.
//!
//! ## Feature Flags
//!
//! The library supports optional features:
//!
//! - **`mqtt`** (default) - Enables MQTT bridge functionality
//! - **`cli`** (default) - Enables CLI binary with colored output
//!
//! ```toml
//! # All features (default)
//! libcps = "0.1.0"
//!
//! # Library only, no MQTT
//! libcps = { version = "0.1.0", default-features = false }
//!
//! # Library with MQTT only (no CLI)
//! libcps = { version = "0.1.0", default-features = false, features = ["mqtt"] }
//! ```
//!
//! ## Type Definitions
//!
//! The library provides types that match the CPS pallet:
//!
//! ```
//! use libcps::types::{NodeId, NodeData};
//!
//! let node_id = NodeId(42);
//! let plain_data = NodeData::from(b"sensor reading".to_vec());
//! let encrypted_data = NodeData::aead_from(vec![1, 2, 3, 4]);
//! ```
//!
//! ## Crates.io Metadata
//!
//! - **Repository**: <https://github.com/airalab/robonomics>
//! - **Documentation**: <https://docs.rs/libcps>
//! - **License**: Apache-2.0
//!
//! ## Safety
//!
//! This crate uses `#![forbid(unsafe_code)]` to ensure memory safety.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod blockchain;
pub mod crypto;
#[cfg(feature = "mqtt")]
pub mod mqtt;
pub mod node;
pub mod types;

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
// Robonomics runtime API generated from WASM
// This macro generates the runtime API code from the WASM runtime file.
// The WASM path is set by build.rs which builds robonomics-runtime as a dependency.
// This ensures the metadata is always in sync with the runtime.
#[subxt::subxt(
    runtime_metadata_path = env!("ROBONOMICS_RUNTIME_WASM"),
    derive_for_type(path = "pallet_robonomics_cps::NodeId", derive = "Copy")
)]
pub mod robonomics_api {}

// Re-export event types for CLI usage
pub use robonomics_api::cps::events::PayloadSet;

// Re-export commonly used types for convenience
pub use blockchain::{Client, Config};
pub use types::{EncryptedData, NodeData, NodeId};
