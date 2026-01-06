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
//! The library implements **sr25519 → XChaCha20-Poly1305** encryption with proper ECDH:
//!
//! ```no_run
//! use libcps::crypto::{SharedSecret, encrypt, decrypt};
//! use schnorrkel::{SecretKey, PublicKey};
//!
//! # fn example() -> anyhow::Result<()> {
//! let sender_secret = SecretKey::from_bytes(&[0u8; 64])?;
//! let receiver_public = [0u8; 32];
//! let plaintext = b"secret message";
//!
//! // Encrypt using high-level API
//! let encrypted = encrypt(plaintext, &sender_secret, &receiver_public)?;
//!
//! // Or use low-level SharedSecret API for key derivation
//! let their_public = PublicKey::from_bytes(&receiver_public)?;
//! let shared = SharedSecret::new(&sender_secret, &their_public)?;
//! let encryption_key = shared.derive_encryption_key()?;
//!
//! // Decrypt
//! let receiver_secret = SecretKey::from_bytes(&[0u8; 64])?;
//! let decrypted = decrypt(&encrypted, &receiver_secret)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## MQTT Bridge
//!
//! Configure MQTT connections for IoT integration:
//!
//! ```no_run
//! use libcps::mqtt::Config as MqttConfig;
//!
//! let mqtt_config = MqttConfig {
//!     broker: "mqtt://localhost:1883".to_string(),
//!     username: Some("user".to_string()),
//!     password: Some("pass".to_string()),
//!     client_id: Some("my-client".to_string()),
//! };
//! ```
//!
//! ## Type Definitions
//!
//! The library provides SCALE-compatible types that match the CPS pallet:
//!
//! ```
//! use libcps::types::{NodeId, NodeData, EncryptedData};
//!
//! let node_id = NodeId(42);
//! let plain_data = NodeData::plain("sensor reading");
//! let encrypted = NodeData::from_encrypted_bytes(vec![1, 2, 3, 4], libcps::crypto::EncryptionAlgorithm::XChaCha20Poly1305);
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
pub mod mqtt;
pub mod node;
pub mod types;

// Generated runtime metadata from subxt
#[allow(dead_code, unused_imports, non_camel_case_types, unreachable_patterns)]
#[allow(clippy::all)]
#[allow(rustdoc::broken_intra_doc_links)]
mod robonomics_runtime;

// Re-export commonly used types for convenience
pub use blockchain::{Client, Config};
pub use types::{EncryptedData, NodeData, NodeId};
