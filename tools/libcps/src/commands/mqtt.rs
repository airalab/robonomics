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
//! MQTT bridge CLI command implementations.
//!
//! This module provides CLI-specific wrappers around the core MQTT bridge
//! functionality, adding colored output, progress messages, and user-friendly
//! error handling.

use crate::display;
use anyhow::Result;
use colored::*;
use libcps::blockchain::Config;
use libcps::crypto::Cipher;
use libcps::mqtt;

/// Subscribe to an MQTT topic and update blockchain node payload (CLI wrapper).
///
/// This function provides a user-friendly CLI interface with colored output
/// and progress messages for the MQTT subscribe bridge.
pub async fn subscribe(
    blockchain_config: &Config,
    cipher: Option<&Cipher>,
    mqtt_config: &mqtt::Config,
    topic: &str,
    node_id: u64,
    receiver_public: Option<[u8; 32]>,
) -> Result<()> {
    display::tree::progress("Connecting to blockchain...");
    
    // Early validation
    let (host, port) = mqtt::parse_mqtt_url(&mqtt_config.broker)?;
    
    display::tree::info(&format!("Connected to {}", blockchain_config.ws_url));
    display::tree::info(&format!("Topic: {}", topic.bright_cyan()));
    display::tree::info(&format!("Node: {}", node_id.to_string().bright_cyan()));

    if let Some(receiver_pub) = receiver_public.as_ref() {
        if let Some(cipher) = cipher {
            display::tree::info(&format!(
                "🔐 Using encryption: {} with {}",
                cipher.algorithm(),
                cipher.scheme()
            ));
            display::tree::info(&format!("🔑 Receiver: {}", hex::encode(receiver_pub)));
        }
    }

    display::tree::progress(&format!("Connecting to MQTT broker {}:{}...", host, port));

    // Create a message handler for CLI output
    let topic_clone = topic.to_string();
    let message_handler = Box::new(move |_t: &str, payload: &[u8]| {
        let payload_str = String::from_utf8_lossy(payload);
        println!(
            "[{}] {} Received from {}: {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            "📥".bright_green(),
            topic_clone.bright_cyan(),
            payload_str.bright_white()
        );
    });

    display::tree::success(&format!(
        "Connected to {}",
        mqtt_config.broker.bright_white()
    ));
    display::tree::info(&format!(
        "📡 Listening for messages on {}...",
        topic.bright_cyan()
    ));

    // Start the bridge with CLI message handler
    mqtt::start_subscribe_bridge(
        blockchain_config,
        cipher,
        mqtt_config,
        topic,
        node_id,
        receiver_public,
        Some(message_handler),
    )
    .await
}

/// Publish blockchain node payload changes to an MQTT topic (CLI wrapper).
///
/// This function provides a user-friendly CLI interface with colored output
/// and progress messages for the MQTT publish bridge.
pub async fn publish(
    blockchain_config: &Config,
    mqtt_config: &mqtt::Config,
    topic: &str,
    node_id: u64,
) -> Result<()> {
    display::tree::progress("Connecting to blockchain...");
    
    // Early validation
    let (host, port) = mqtt::parse_mqtt_url(&mqtt_config.broker)?;

    display::tree::info(&format!("Connected to {}", blockchain_config.ws_url));

    display::tree::progress(&format!("Connecting to MQTT broker {}:{}...", host, port));

    display::tree::success(&format!(
        "Connected to {}",
        mqtt_config.broker.bright_white()
    ));
    display::tree::info(&format!(
        "🔄 Monitoring node {} payload on each block...",
        node_id.to_string().bright_cyan()
    ));

    // Start the bridge
    mqtt::start_publish_bridge(blockchain_config, mqtt_config, topic, node_id).await
}
