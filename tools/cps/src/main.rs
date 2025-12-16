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
//! CPS CLI - Command-line interface for Robonomics CPS pallet.
//!
//! This binary provides a beautiful, user-friendly CLI for managing
//! cyber-physical systems on the Robonomics blockchain.

use anyhow::Result;
use clap::{Parser, Subcommand};

// Import from the library
use libcps::{blockchain, mqtt, types};

// CLI-specific modules (display and commands)
mod commands;
mod display;

#[derive(Parser)]
#[command(name = "cps")]
#[command(version, about = "🌳 Beautiful CLI for Robonomics CPS (Cyber-Physical Systems)", long_about = None)]
struct Cli {
    /// WebSocket URL for blockchain connection
    #[arg(long, env = "ROBONOMICS_WS_URL", default_value = "ws://localhost:9944")]
    ws_url: String,

    /// Account secret URI (e.g., //Alice, //Bob, or seed phrase)
    #[arg(long, env = "ROBONOMICS_SURI")]
    suri: Option<String>,

    /// MQTT broker URL
    #[arg(
        long,
        env = "ROBONOMICS_MQTT_BROKER",
        default_value = "mqtt://localhost:1883"
    )]
    mqtt_broker: String,

    /// MQTT username
    #[arg(long, env = "ROBONOMICS_MQTT_USERNAME")]
    mqtt_username: Option<String>,

    /// MQTT password
    #[arg(long, env = "ROBONOMICS_MQTT_PASSWORD")]
    mqtt_password: Option<String>,

    /// MQTT client ID
    #[arg(long, env = "ROBONOMICS_MQTT_CLIENT_ID")]
    mqtt_client_id: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Display node information and its children in a beautiful tree format
    Show {
        /// Node ID to display
        node_id: u64,

        /// Attempt to decrypt encrypted data
        #[arg(long)]
        decrypt: bool,
    },

    /// Create a new node (root or child)
    Create {
        /// Parent node ID (omit for root node)
        #[arg(long)]
        parent: Option<u64>,

        /// Metadata (configuration data)
        #[arg(long)]
        meta: Option<String>,

        /// Payload (operational data)
        #[arg(long)]
        payload: Option<String>,

        /// Encrypt the data
        #[arg(long)]
        encrypt: bool,

        /// Encryption algorithm (xchacha20, aesgcm256, chacha20)
        #[arg(long, default_value = "xchacha20")]
        cipher: String,

        /// Keypair type for encryption (sr25519, ed25519)
        #[arg(long, default_value = "sr25519", value_parser = clap::value_parser!(libcps::crypto::KeypairType))]
        keypair_type: libcps::crypto::KeypairType,
    },

    /// Update node metadata
    SetMeta {
        /// Node ID
        node_id: u64,

        /// New metadata
        data: String,

        /// Encrypt the data
        #[arg(long)]
        encrypt: bool,

        /// Encryption algorithm (xchacha20, aesgcm256, chacha20)
        #[arg(long, default_value = "xchacha20")]
        cipher: String,

        /// Keypair type for encryption (sr25519, ed25519)
        #[arg(long, default_value = "sr25519", value_parser = clap::value_parser!(libcps::crypto::KeypairType))]
        keypair_type: libcps::crypto::KeypairType,
    },

    /// Update node payload
    SetPayload {
        /// Node ID
        node_id: u64,

        /// New payload
        data: String,

        /// Encrypt the data
        #[arg(long)]
        encrypt: bool,

        /// Encryption algorithm (xchacha20, aesgcm256, chacha20)
        #[arg(long, default_value = "xchacha20")]
        cipher: String,

        /// Keypair type for encryption (sr25519, ed25519)
        #[arg(long, default_value = "sr25519", value_parser = clap::value_parser!(libcps::crypto::KeypairType))]
        keypair_type: libcps::crypto::KeypairType,
    },

    /// Move a node to a new parent
    Move {
        /// Node ID to move
        node_id: u64,

        /// New parent node ID
        new_parent_id: u64,
    },

    /// Delete a node (must have no children)
    Remove {
        /// Node ID to remove
        node_id: u64,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// MQTT bridge commands
    #[command(subcommand)]
    Mqtt(MqttCommands),
}

#[derive(Subcommand)]
enum MqttCommands {
    /// Subscribe to MQTT topic and update node payload with received messages
    Subscribe {
        /// MQTT topic to subscribe to
        topic: String,

        /// Node ID to update
        node_id: u64,

        /// Encrypt messages before storing
        #[arg(long)]
        encrypt: bool,

        /// Encryption algorithm (xchacha20, aesgcm256, chacha20)
        #[arg(long, default_value = "xchacha20")]
        cipher: String,

        /// Keypair type for encryption (sr25519, ed25519)
        #[arg(long, default_value = "sr25519", value_parser = clap::value_parser!(libcps::crypto::KeypairType))]
        keypair_type: libcps::crypto::KeypairType,
    },

    /// Publish node payload changes to MQTT topic
    Publish {
        /// MQTT topic to publish to
        topic: String,

        /// Node ID to monitor
        node_id: u64,

        /// Polling interval in seconds
        #[arg(long, default_value = "5")]
        interval: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Create blockchain config
    let blockchain_config = blockchain::Config {
        ws_url: cli.ws_url.clone(),
        suri: cli.suri.clone(),
    };

    // Create MQTT config
    let mqtt_config = mqtt::Config {
        broker: cli.mqtt_broker.clone(),
        username: cli.mqtt_username.clone(),
        password: cli.mqtt_password.clone(),
        client_id: cli.mqtt_client_id.clone(),
    };

    // Execute commands
    match cli.command {
        Commands::Show { node_id, decrypt } => {
            commands::show::execute(&blockchain_config, node_id, decrypt).await?;
        }
        Commands::Create {
            parent,
            meta,
            payload,
            encrypt,
            cipher,
            keypair_type,
        } => {
            commands::create::execute(
                &blockchain_config,
                parent,
                meta,
                payload,
                encrypt,
                &cipher,
                keypair_type,
            )
            .await?;
        }
        Commands::SetMeta {
            node_id,
            data,
            encrypt,
            cipher,
            keypair_type,
        } => {
            commands::set_meta::execute(
                &blockchain_config,
                node_id,
                data,
                encrypt,
                &cipher,
                keypair_type,
            )
            .await?;
        }
        Commands::SetPayload {
            node_id,
            data,
            encrypt,
            cipher,
            keypair_type,
        } => {
            commands::set_payload::execute(
                &blockchain_config,
                node_id,
                data,
                encrypt,
                &cipher,
                keypair_type,
            )
            .await?;
        }
        Commands::Move {
            node_id,
            new_parent_id,
        } => {
            commands::move_node::execute(&blockchain_config, node_id, new_parent_id).await?;
        }
        Commands::Remove { node_id, force } => {
            commands::remove::execute(&blockchain_config, node_id, force).await?;
        }
        Commands::Mqtt(mqtt_cmd) => match mqtt_cmd {
            MqttCommands::Subscribe {
                topic,
                node_id,
                encrypt,
                cipher,
                keypair_type,
            } => {
                commands::mqtt::subscribe(
                    &blockchain_config,
                    &mqtt_config,
                    &topic,
                    node_id,
                    encrypt,
                    &cipher,
                    keypair_type,
                )
                .await?;
            }
            MqttCommands::Publish {
                topic,
                node_id,
                interval,
            } => {
                commands::mqtt::publish(
                    &blockchain_config,
                    &mqtt_config,
                    &topic,
                    node_id,
                    interval,
                )
                .await?;
            }
        },
    }

    Ok(())
}
