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
//! MQTT bridge configuration and implementation.
//!
//! This module provides configuration types and bridge implementations for
//! connecting to MQTT brokers and bridging messages between MQTT topics
//! and blockchain nodes.
//!
//! # Examples
//!
//! ## Configuration
//!
//! ```
//! use libcps::mqtt::Config;
//!
//! let config = Config {
//!     broker: "mqtt://localhost:1883".to_string(),
//!     username: Some("user".to_string()),
//!     password: Some("pass".to_string()),
//!     client_id: Some("my-client".to_string()),
//!     blockchain: None,
//!     subscribe: Vec::new(),
//!     publish: Vec::new(),
//! };
//! ```
//!
//! ## Subscribe Bridge
//!
//! ```no_run
//! use libcps::{mqtt, Config as BlockchainConfig};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let blockchain_config = BlockchainConfig {
//!     ws_url: "ws://localhost:9944".to_string(),
//!     suri: Some("//Alice".to_string()),
//! };
//!
//! let mqtt_config = mqtt::Config {
//!     broker: "mqtt://localhost:1883".to_string(),
//!     username: None,
//!     password: None,
//!     client_id: None,
//!     blockchain: None,
//!     subscribe: Vec::new(),
//!     publish: Vec::new(),
//! };
//!
//! // Using Config method API
//! mqtt_config.subscribe(
//!     &blockchain_config,
//!     None,
//!     "sensors/temp",
//!     1,
//!     None,
//!     None,
//!     None,
//! ).await?;
//! # Ok(())
//! # }
//! ```

pub mod bridge;

pub use bridge::{
    parse_mqtt_url, BlockchainConfigData, Config, MessageHandler, PublishConfig, PublishHandler,
    SubscribeConfig,
};
