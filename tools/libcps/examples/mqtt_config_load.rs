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
//! Example: Loading MQTT configuration from a TOML file
//!
//! This example demonstrates how to load and validate an MQTT bridge
//! configuration file.
//!
//! Run with: cargo run --example mqtt_config_load

use libcps::mqtt::Config;

fn main() -> anyhow::Result<()> {
    println!("MQTT Configuration File Loading Example");
    println!("========================================\n");

    // Load configuration from file
    println!("Loading configuration from examples/mqtt_config.toml...");
    let config = Config::from_file("examples/mqtt_config.toml")?;
    
    println!("✅ Configuration loaded successfully!\n");
    
    // Display broker configuration
    println!("📡 MQTT Broker Configuration:");
    println!("   Broker: {}", config.broker);
    if let Some(username) = &config.username {
        println!("   Username: {}", username);
    }
    if let Some(client_id) = &config.client_id {
        println!("   Client ID: {}", client_id);
    }
    println!();
    
    // Display blockchain configuration
    if let Some(blockchain) = &config.blockchain {
        println!("🔗 Blockchain Configuration:");
        println!("   WS URL: {}", blockchain.ws_url);
        if let Some(suri) = &blockchain.suri {
            println!("   SURI: {}", suri);
        }
        println!();
    }
    
    // Display subscribe topics
    println!("📥 Subscribe Topics ({}):", config.subscribe.len());
    for (i, sub) in config.subscribe.iter().enumerate() {
        println!("   {}. {} -> Node ID {}", i + 1, sub.topic, sub.node_id);
        if let Some(receiver) = &sub.receiver_public {
            println!("      🔐 Encrypted (receiver: {}...)", &receiver[..20]);
            if let Some(cipher) = &sub.cipher {
                println!("      Cipher: {}", cipher);
            }
            if let Some(scheme) = &sub.scheme {
                println!("      Scheme: {}", scheme);
            }
        }
    }
    println!();
    
    // Display publish topics
    println!("📤 Publish Topics ({}):", config.publish.len());
    for (i, pub_cfg) in config.publish.iter().enumerate() {
        println!("   {}. {} <- Node ID {}", i + 1, pub_cfg.topic, pub_cfg.node_id);
    }
    println!();
    
    println!("To start all bridges, run:");
    println!("  cps mqtt start -c examples/mqtt_config.toml");
    println!();
    println!("Or from library code:");
    println!("  config.start().await?;");
    
    Ok(())
}
