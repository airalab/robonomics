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
//! MQTT bridge command implementations.

use crate::display;
use anyhow::{anyhow, Result};
use colored::*;
use libcps::blockchain::{Client, Config};
use libcps::crypto::Cipher;
use libcps::mqtt;
use libcps::node::Node;
use libcps::types::NodeData;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use tokio::time::{sleep, Duration};

/// Parse MQTT broker URL to extract host and port
fn parse_mqtt_url(url: &str) -> Result<(String, u16)> {
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

pub async fn subscribe(
    blockchain_config: &Config,
    cipher: Option<&Cipher>,
    mqtt_config: &mqtt::Config,
    topic: &str,
    node_id: u64,
    receiver_public: Option<[u8; 32]>,
) -> Result<()> {
    display::tree::progress("Connecting to blockchain...");
    let client = Client::new(blockchain_config).await?;
    let _keypair = client.require_keypair()?;

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

    // Parse MQTT broker URL
    let (host, port) = parse_mqtt_url(&mqtt_config.broker)?;
    
    display::tree::progress(&format!("Connecting to MQTT broker {}:{}...", host, port));

    // Configure MQTT client
    let client_id = mqtt_config
        .client_id
        .clone()
        .unwrap_or_else(|| format!("cps-sub-{}", node_id));
    
    let mut mqttoptions = MqttOptions::new(client_id, host, port);
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    
    // Set credentials if provided
    if let Some(username) = &mqtt_config.username {
        mqttoptions.set_credentials(
            username,
            mqtt_config.password.as_deref().unwrap_or(""),
        );
    }

    // Create MQTT client with eventloop
    let (mqtt_client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // Subscribe to topic
    mqtt_client
        .subscribe(topic, QoS::AtMostOnce)
        .await
        .map_err(|e| anyhow!("Failed to subscribe to topic: {}", e))?;

    display::tree::success(&format!("Connected to {}", mqtt_config.broker.bright_white()));
    display::tree::info(&format!(
        "📡 Listening for messages on {}...",
        topic.bright_cyan()
    ));

    // Create Node handle for updates
    let node = Node::new(&client, node_id);

    // Process MQTT events
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(publish))) => {
                let payload_str = String::from_utf8_lossy(&publish.payload);
                
                println!(
                    "[{}] {} Received from {}: {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    "📥".bright_green(),
                    topic.bright_cyan(),
                    payload_str.bright_white()
                );

                // Prepare node data (encrypt if needed)
                let node_data = if let Some(receiver_pub) = receiver_public.as_ref() {
                    if let Some(cipher) = cipher {
                        let encrypted_bytes = cipher.encrypt(&publish.payload, receiver_pub)?;
                        NodeData::from_encrypted_bytes(encrypted_bytes, cipher.algorithm())
                    } else {
                        NodeData::from(payload_str.to_string())
                    }
                } else {
                    NodeData::from(payload_str.to_string())
                };

                // Update node payload on blockchain
                match node.set_payload(Some(node_data)).await {
                    Ok(_) => {
                        println!(
                            "[{}] {} Updated blockchain node {}",
                            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                            "✅".green(),
                            node_id.to_string().bright_cyan()
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "[{}] {} Failed to update node: {}",
                            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                            "❌".red(),
                            e.to_string().red()
                        );
                    }
                }
            }
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                display::tree::info(&format!("📡 Connected to MQTT broker"));
            }
            Ok(Event::Incoming(Packet::SubAck(_))) => {
                // Subscription acknowledged
            }
            Ok(_) => {
                // Other events, ignore
            }
            Err(e) => {
                display::tree::warning(&format!("Connection error: {}. Reconnecting...", e));
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

pub async fn publish(
    blockchain_config: &Config,
    mqtt_config: &mqtt::Config,
    topic: &str,
    node_id: u64,
    interval: u64,
) -> Result<()> {
    display::tree::progress("Connecting to blockchain...");
    let client = Client::new(blockchain_config).await?;

    display::tree::info(&format!("Connected to {}", blockchain_config.ws_url));
    
    // Parse MQTT broker URL
    let (host, port) = parse_mqtt_url(&mqtt_config.broker)?;
    
    display::tree::progress(&format!("Connecting to MQTT broker {}:{}...", host, port));

    // Configure MQTT client
    let client_id = mqtt_config
        .client_id
        .clone()
        .unwrap_or_else(|| format!("cps-pub-{}", node_id));
    
    let mut mqttoptions = MqttOptions::new(client_id, host, port);
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    
    // Set credentials if provided
    if let Some(username) = &mqtt_config.username {
        mqttoptions.set_credentials(
            username,
            mqtt_config.password.as_deref().unwrap_or(""),
        );
    }

    // Create MQTT client
    let (mqtt_client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // Spawn task to handle MQTT events (for auto-reconnect)
    tokio::spawn(async move {
        loop {
            if let Err(e) = eventloop.poll().await {
                eprintln!(
                    "[{}] {} MQTT connection error: {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    "⚠️".yellow(),
                    e.to_string().yellow()
                );
                sleep(Duration::from_secs(5)).await;
            }
        }
    });

    display::tree::success(&format!("Connected to {}", mqtt_config.broker.bright_white()));
    display::tree::info(&format!(
        "🔄 Monitoring node {} payload every {} seconds...",
        node_id.to_string().bright_cyan(),
        interval
    ));

    // Create Node handle for querying
    let node = Node::new(&client, node_id);

    let mut last_payload: Option<Vec<u8>> = None;

    loop {
        sleep(Duration::from_secs(interval)).await;

        // Query node information
        match node.query().await {
            Ok(node_info) => {
                if let Some(payload) = node_info.payload {
                    // Convert NodeData to bytes for comparison
                    let payload_bytes = format!("{:?}", payload).into_bytes();

                    if last_payload.as_ref() != Some(&payload_bytes) {
                        // Payload changed, publish to MQTT
                        
                        // Try to extract the actual data from NodeData
                        let data = extract_node_data(&payload);

                        match mqtt_client
                            .publish(topic, QoS::AtMostOnce, false, data.as_bytes())
                            .await
                        {
                            Ok(_) => {
                                println!(
                                    "[{}] {} Published to {}: {}",
                                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                                    "📤".bright_blue(),
                                    topic.bright_cyan(),
                                    if data.len() > 100 {
                                        format!("{}...", &data[..97])
                                    } else {
                                        data.clone()
                                    }
                                    .bright_white()
                                );
                            }
                            Err(e) => {
                                eprintln!(
                                    "[{}] {} Failed to publish: {}",
                                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                                    "❌".red(),
                                    e.to_string().red()
                                );
                            }
                        }

                        last_payload = Some(payload_bytes);
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "[{}] {} Failed to query node: {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    "❌".red(),
                    e.to_string().red()
                );
            }
        }
    }
}

/// Extract readable data from NodeData
fn extract_node_data(node_data: &NodeData) -> String {
    // NodeData is an enum with Plain(BoundedVec<u8>) and Encrypted variants
    // We'll use debug format and try to extract the actual data
    let debug_str = format!("{:?}", node_data);
    
    // Try to extract bytes from Plain variant
    if debug_str.starts_with("Plain(") {
        // Try to parse as UTF-8
        if let Some(start) = debug_str.find('[') {
            if let Some(end) = debug_str.rfind(']') {
                let bytes_str = &debug_str[start + 1..end];
                let bytes: Vec<u8> = bytes_str
                    .split(',')
                    .filter_map(|s| s.trim().parse::<u8>().ok())
                    .collect();
                if let Ok(s) = String::from_utf8(bytes) {
                    return s;
                }
            }
        }
    }
    
    // For encrypted or unparseable data, return as-is
    debug_str
}
