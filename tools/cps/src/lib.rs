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
//!
//! ## Encryption
//!
//! The library implements **sr25519 → XChaCha20-Poly1305** encryption:
//!
//! ```no_run
//! use libcps::crypto::{encrypt, decrypt};
//! use schnorrkel::SecretKey;
//!
//! # fn example() -> anyhow::Result<()> {
//! let sender_secret = SecretKey::from_bytes(&[0u8; 64])?;
//! let receiver_public = [0u8; 32];
//! let plaintext = b"secret message";
//!
//! // Encrypt
//! let encrypted = encrypt(plaintext, &sender_secret, &receiver_public)?;
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
//! The library provides types that match the CPS pallet:
//!
//! ```
//! use libcps::types::{NodeId, NodeData, EncryptedData};
//!
//! let node_id = NodeId(42);
//! let plain_data = NodeData::plain("sensor reading");
//! let encrypted_data = NodeData::encrypted_xchacha(vec![1, 2, 3, 4]);
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
pub mod types;

// Re-export commonly used types for convenience
pub use blockchain::{Client, Config};
pub use types::{EncryptedData, Node, NodeData, NodeId};
