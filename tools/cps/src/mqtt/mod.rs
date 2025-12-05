//! MQTT bridge configuration and types.
//!
//! This module provides configuration types for connecting to MQTT brokers
//! and bridging messages between MQTT topics and blockchain nodes.
//!
//! # Examples
//!
//! ```
//! use libcps::mqtt::Config;
//!
//! let config = Config {
//!     broker: "mqtt://localhost:1883".to_string(),
//!     username: Some("user".to_string()),
//!     password: Some("pass".to_string()),
//!     client_id: Some("my-client".to_string()),
//! };
//! ```

pub mod bridge;

pub use bridge::Config;
