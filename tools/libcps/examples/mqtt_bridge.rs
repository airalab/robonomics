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
//! Example: Using MQTT bridge from library code
//!
//! This example demonstrates how to use the MQTT bridge functionality
//! programmatically from your own Rust application.
//!
//! Run with: cargo run --example mqtt_bridge

use libcps::{blockchain::Config as BlockchainConfig, mqtt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("MQTT Bridge Library Example");
    println!("============================\n");

    // Configure blockchain connection
    let blockchain_config = BlockchainConfig {
        ws_url: "ws://localhost:9944".to_string(),
        suri: Some("//Alice".to_string()),
    };

    // Configure MQTT connection
    let mqtt_config = mqtt::Config {
        broker: "mqtt://localhost:1883".to_string(),
        username: None,
        password: None,
        client_id: Some("libcps-example".to_string()),
        blockchain: None,
        subscribe: Vec::new(),
        publish: Vec::new(),
    };

    // Parse and display MQTT broker details
    let (host, port) = mqtt::parse_mqtt_url(&mqtt_config.broker)?;
    println!("MQTT Broker: {}:{}", host, port);
    println!("Blockchain: {}", blockchain_config.ws_url);
    println!();

    // Example 1: Subscribe Bridge
    println!("Starting subscribe bridge...");
    println!("This will listen to 'sensors/temp' and update node 1");

    // Create a custom message handler
    let handler = Box::new(|topic: &str, payload: &[u8]| {
        println!("📥 Received on {}: {:?}", topic, payload);
    });

    mqtt_config
        .subscribe(
            &blockchain_config,
            None, // No encryption
            "sensors/temp",
            1,    // node_id
            None, // No receiver public key
            None, // No algorithm (no encryption)
            Some(handler),
        )
        .await?;

    Ok(())
}
