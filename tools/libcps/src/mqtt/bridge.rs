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
//! MQTT bridge implementation for connecting blockchain nodes to MQTT brokers.
//!
//! This module provides the core MQTT bridge functionality that can be used
//! both from the CLI and as a library. It supports bidirectional communication:
//! - **Subscribe mode**: Listen to MQTT topics and update blockchain node payloads
//! - **Publish mode**: Monitor blockchain events and publish to MQTT topics
//!
//! # Examples
//!
//! ## Basic Configuration
//!
//! ```
//! use libcps::mqtt::Config;
//!
//! // Anonymous connection
//! let config = Config {
//!     broker: "mqtt://localhost:1883".to_string(),
//!     username: None,
//!     password: None,
//!     client_id: None,
//! };
//!
//! // Authenticated connection
//! let config_auth = Config {
//!     broker: "mqtt://broker.example.com:1883".to_string(),
//!     username: Some("myuser".to_string()),
//!     password: Some("mypass".to_string()),
//!     client_id: Some("cps-client".to_string()),
//! };
//! ```
//!
//! ## Subscribe Bridge (Library Usage)
//!
//! ```no_run
//! use libcps::{Client, Config as BlockchainConfig, mqtt};
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
//!     client_id: Some("cps-subscriber".to_string()),
//! };
//!
//! // Start subscribing to MQTT and updating blockchain
//! mqtt::start_subscribe_bridge(
//!     &blockchain_config,
//!     None, // No encryption
//!     &mqtt_config,
//!     "sensors/temperature",
//!     42, // node_id
//!     None, // No receiver public key
//!     None, // No custom message handler
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Publish Bridge (Library Usage)
//!
//! ```no_run
//! use libcps::{Client, Config as BlockchainConfig, mqtt};
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
//!     client_id: Some("cps-publisher".to_string()),
//! };
//!
//! // Start publishing blockchain updates to MQTT
//! mqtt::start_publish_bridge(
//!     &blockchain_config,
//!     &mqtt_config,
//!     "actuators/valve",
//!     42, // node_id
//! ).await?;
//! # Ok(())
//! # }
//! ```

use crate::blockchain::{Client, Config as BlockchainConfig};
use crate::crypto::Cipher;
use crate::node::Node;
use crate::types::{EncryptedData, NodeData};
use crate::PayloadSet;
use anyhow::{anyhow, Result};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use tokio::time::{sleep, Duration};

/// Configuration for MQTT broker connection.
///
/// This configuration is used to establish connections to MQTT brokers
/// for IoT device integration.
#[derive(Clone, Debug)]
pub struct Config {
    /// MQTT broker URL (e.g., "mqtt://localhost:1883")
    pub broker: String,
    /// Optional username for authentication
    pub username: Option<String>,
    /// Optional password for authentication
    pub password: Option<String>,
    /// Optional client ID for MQTT connection
    pub client_id: Option<String>,
}

/// Constants for MQTT operation
const MQTT_RECONNECT_DELAY_SECS: u64 = 5;

/// Parse MQTT broker URL to extract host and port.
///
/// Supports both `mqtt://` and `mqtts://` URL schemes.
/// Defaults to port 1883 if not specified.
///
/// # Examples
///
/// ```
/// # use libcps::mqtt::parse_mqtt_url;
/// let (host, port) = parse_mqtt_url("mqtt://localhost:1883").unwrap();
/// assert_eq!(host, "localhost");
/// assert_eq!(port, 1883);
///
/// let (host, port) = parse_mqtt_url("mqtt://broker.example.com").unwrap();
/// assert_eq!(host, "broker.example.com");
/// assert_eq!(port, 1883);
/// ```
pub fn parse_mqtt_url(url: &str) -> Result<(String, u16)> {
    let url = url.trim();

    // Remove mqtt:// or mqtts:// prefix if present
    let url = url
        .strip_prefix("mqtt://")
        .or_else(|| url.strip_prefix("mqtts://"))
        .unwrap_or(url);

    // Split host and port
    if let Some((host, port_str)) = url.split_once(':') {
        let port = port_str
            .parse::<u16>()
            .map_err(|_| anyhow!("Invalid port in MQTT URL: {}", port_str))?;
        Ok((host.to_string(), port))
    } else {
        // Default to port 1883 if not specified
        Ok((url.to_string(), 1883))
    }
}

/// Extract readable data from NodeData.
///
/// Converts NodeData to a string representation:
/// - Plain data: UTF-8 string or binary indicator
/// - Encrypted data: Indicates encryption with size
///
/// # Examples
///
/// ```
/// # use libcps::types::NodeData;
/// # use libcps::mqtt::extract_node_data;
/// let plain_data = NodeData::from("Hello, World!".to_string());
/// assert_eq!(extract_node_data(&plain_data), "Hello, World!");
/// ```
pub fn extract_node_data(node_data: &NodeData) -> String {
    match node_data {
        NodeData::Plain(bounded_vec) => {
            // Try to convert bytes to UTF-8 string
            String::from_utf8(bounded_vec.0.clone())
                .unwrap_or_else(|_| format!("[Binary data: {} bytes]", bounded_vec.0.len()))
        }
        NodeData::Encrypted(EncryptedData::Aead(bounded_vec)) => {
            // For encrypted data, indicate it's encrypted
            let size = bounded_vec.0.len();
            format!("[Encrypted data: {} bytes]", size)
        }
    }
}

/// Optional callback type for custom message handling in subscribe bridge.
///
/// When provided, this callback is called for each received MQTT message
/// before it's sent to the blockchain. Can be used for logging, validation,
/// or custom processing.
pub type MessageHandler = Box<dyn Fn(&str, &[u8]) + Send + Sync>;

/// Optional callback type for publish notifications.
///
/// When provided, this callback is called after successfully publishing
/// a message to MQTT. Can be used for logging or custom tracking.
/// 
/// Arguments: (topic, block_number, data)
pub type PublishHandler = Box<dyn Fn(&str, u32, &str) + Send + Sync>;

/// Start an MQTT subscribe bridge that listens to a topic and updates blockchain node payload.
///
/// This function creates a long-running bridge that:
/// 1. Connects to the specified MQTT broker
/// 2. Subscribes to the given topic
/// 3. On each message, optionally encrypts it and updates the blockchain node payload
/// 4. Automatically reconnects on connection failures
///
/// # Arguments
///
/// * `blockchain_config` - Configuration for blockchain connection
/// * `cipher` - Optional cipher for encrypting messages before sending to blockchain
/// * `mqtt_config` - MQTT broker configuration
/// * `topic` - MQTT topic to subscribe to
/// * `node_id` - Blockchain node ID to update
/// * `receiver_public` - Optional public key for encryption (required if cipher is provided)
/// * `message_handler` - Optional callback for custom message processing
///
/// # Returns
///
/// This function runs indefinitely and only returns on fatal errors.
///
/// # Examples
///
/// ```no_run
/// # use libcps::{mqtt, Config as BlockchainConfig};
/// # async fn example() -> anyhow::Result<()> {
/// let blockchain_config = BlockchainConfig {
///     ws_url: "ws://localhost:9944".to_string(),
///     suri: Some("//Alice".to_string()),
/// };
///
/// let mqtt_config = mqtt::Config {
///     broker: "mqtt://localhost:1883".to_string(),
///     username: None,
///     password: None,
///     client_id: None,
/// };
///
/// mqtt::start_subscribe_bridge(
///     &blockchain_config,
///     None,
///     &mqtt_config,
///     "sensors/temp",
///     1,
///     None,
///     None,
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn start_subscribe_bridge(
    blockchain_config: &BlockchainConfig,
    cipher: Option<&Cipher>,
    mqtt_config: &Config,
    topic: &str,
    node_id: u64,
    receiver_public: Option<[u8; 32]>,
    message_handler: Option<MessageHandler>,
) -> Result<()> {
    // Connect to blockchain
    let client = Client::new(blockchain_config).await?;
    let _keypair = client.require_keypair()?;

    // Parse MQTT broker URL
    let (host, port) = parse_mqtt_url(&mqtt_config.broker)?;

    // Configure MQTT client
    let client_id = mqtt_config
        .client_id
        .clone()
        .unwrap_or_else(|| format!("cps-sub-{}", node_id));

    let mut mqttoptions = MqttOptions::new(client_id, host, port);
    mqttoptions.set_keep_alive(Duration::from_secs(30));

    // Set credentials if provided
    if let Some(username) = &mqtt_config.username {
        mqttoptions.set_credentials(username, mqtt_config.password.as_deref().unwrap_or(""));
    }

    // Create MQTT client with eventloop
    let (mqtt_client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // Subscribe to topic
    mqtt_client
        .subscribe(topic, QoS::AtMostOnce)
        .await
        .map_err(|e| anyhow!("Failed to subscribe to topic: {}", e))?;

    // Create Node handle for updates
    let node = Node::new(&client, node_id);

    // Process MQTT events
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(publish))) => {
                // Call custom handler if provided
                if let Some(ref handler) = message_handler {
                    handler(topic, &publish.payload);
                }

                // Prepare node data (encrypt if needed)
                let node_data = match (receiver_public.as_ref(), cipher) {
                    (Some(receiver_pub), Some(cipher)) => {
                        match cipher.encrypt(&publish.payload, receiver_pub) {
                            Ok(encrypted_bytes) => NodeData::aead_from(encrypted_bytes),
                            Err(e) => {
                                // Encryption failed, log error and continue
                                eprintln!("Failed to encrypt message: {}", e);
                                continue;
                            }
                        }
                    }
                    _ => {
                        let payload_str = String::from_utf8_lossy(&publish.payload);
                        NodeData::from(payload_str.to_string())
                    }
                };

                // Update node payload on blockchain
                if let Err(e) = node.set_payload(Some(node_data)).await {
                    // Blockchain update failed, log error and continue
                    eprintln!("Failed to update node payload: {}", e);
                }
            }
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                // Connected to MQTT broker
            }
            Ok(Event::Incoming(Packet::SubAck(_))) => {
                // Subscription acknowledged
            }
            Ok(_) => {
                // Other events, ignore
            }
            Err(_e) => {
                // Connection error, wait and retry
                sleep(Duration::from_secs(MQTT_RECONNECT_DELAY_SECS)).await;
            }
        }
    }
}

/// Start an MQTT publish bridge that monitors blockchain and publishes to MQTT topic.
///
/// This function creates a long-running bridge that:
/// 1. Connects to blockchain and MQTT broker
/// 2. Subscribes to finalized blocks
/// 3. Monitors for PayloadSet events for the specified node
/// 4. Publishes node payload changes to the MQTT topic
/// 5. Automatically handles MQTT reconnections
///
/// # Arguments
///
/// * `blockchain_config` - Configuration for blockchain connection
/// * `mqtt_config` - MQTT broker configuration
/// * `topic` - MQTT topic to publish to
/// * `node_id` - Blockchain node ID to monitor
/// * `publish_handler` - Optional callback for publish notifications
///
/// # Returns
///
/// This function runs indefinitely and only returns on fatal errors.
///
/// # Examples
///
/// ```no_run
/// # use libcps::{mqtt, Config as BlockchainConfig};
/// # async fn example() -> anyhow::Result<()> {
/// let blockchain_config = BlockchainConfig {
///     ws_url: "ws://localhost:9944".to_string(),
///     suri: Some("//Alice".to_string()),
/// };
///
/// let mqtt_config = mqtt::Config {
///     broker: "mqtt://localhost:1883".to_string(),
///     username: None,
///     password: None,
///     client_id: None,
/// };
///
/// mqtt::start_publish_bridge(
///     &blockchain_config,
///     &mqtt_config,
///     "actuators/status",
///     1,
///     None,
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn start_publish_bridge(
    blockchain_config: &BlockchainConfig,
    mqtt_config: &Config,
    topic: &str,
    node_id: u64,
    publish_handler: Option<PublishHandler>,
) -> Result<()> {
    // Connect to blockchain
    let client = Client::new(blockchain_config).await?;

    // Parse MQTT broker URL
    let (host, port) = parse_mqtt_url(&mqtt_config.broker)?;

    // Configure MQTT client
    let client_id = mqtt_config
        .client_id
        .clone()
        .unwrap_or_else(|| format!("cps-pub-{}", node_id));

    let mut mqttoptions = MqttOptions::new(client_id, host, port);
    mqttoptions.set_keep_alive(Duration::from_secs(30));

    // Set credentials if provided
    if let Some(username) = &mqtt_config.username {
        mqttoptions.set_credentials(username, mqtt_config.password.as_deref().unwrap_or(""));
    }

    // Create MQTT client
    let (mqtt_client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // Create shutdown channel for graceful termination of background task
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);

    // Spawn task to handle MQTT events (for auto-reconnect)
    let eventloop_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    // Shutdown signal received, exit gracefully
                    break;
                }
                result = eventloop.poll() => {
                    if let Err(_e) = result {
                        // Connection error, wait and retry
                        sleep(Duration::from_secs(MQTT_RECONNECT_DELAY_SECS)).await;
                    }
                }
            }
        }
    });

    // Create Node handle for querying
    let node = Node::new(&client, node_id);

    // Subscribe to finalized blocks
    let mut blocks_sub = client
        .api
        .blocks()
        .subscribe_finalized()
        .await
        .map_err(|e| anyhow!("Failed to subscribe to finalized blocks: {}", e))?;

    // Monitor each block for PayloadSet events
    while let Some(block_result) = blocks_sub.next().await {
        let block = match block_result {
            Ok(b) => b,
            Err(_e) => {
                continue;
            }
        };

        // Check events in this block for PayloadSet events related to our node
        let events = match block.events().await {
            Ok(e) => e,
            Err(_e) => {
                continue;
            }
        };

        // Look for PayloadSet events for our node
        let payload_set_events = events.find::<PayloadSet>();

        let mut payload_updated = false;
        for event in payload_set_events {
            match event {
                Ok(payload_event) => {
                    // Check if this event is for our node
                    if payload_event.0 .0 == node_id {
                        payload_updated = true;
                        break;
                    }
                }
                Err(_e) => {
                    // Failed to decode event, skip
                }
            }
        }

        // Only query and publish if the payload was actually updated
        if payload_updated {
            match node.query_at(block.hash()).await {
                Ok(node_info) => {
                    if let Some(payload) = node_info.payload {
                        // Extract the actual data from NodeData
                        let data = extract_node_data(&payload);
                        let block_number = block.number();

                        // Publish to MQTT
                        match mqtt_client
                            .publish(topic, QoS::AtMostOnce, false, data.as_bytes())
                            .await
                        {
                            Ok(_) => {
                                // Call publish handler if provided
                                if let Some(ref handler) = publish_handler {
                                    handler(topic, block_number, &data);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to publish to MQTT: {}", e);
                            }
                        }
                    }
                }
                Err(_e) => {
                    // Failed to query node, skip
                }
            }
        }
    }

    // Signal shutdown to the background MQTT event loop task
    let _ = shutdown_tx.send(());

    // Wait for the background task to finish (with timeout to avoid hanging)
    let _ = tokio::time::timeout(Duration::from_secs(5), eventloop_handle).await;

    Ok(())
}
