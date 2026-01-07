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
use sp_core::crypto::Ss58Codec;
use std::str::FromStr;

// Import from the library
use libcps::{blockchain, mqtt};

// CLI-specific modules (display and commands)
mod commands;
mod display;

/// Helper function to parse receiver public key from SS58 address or hex encoding.
/// Uses AccountId32 which supports both Sr25519 and Ed25519 (same 32-byte public key length).
fn parse_receiver_public_key(addr_or_hex: &str) -> Result<[u8; 32]> {
    // Try SS58 decoding with AccountId32 (works for both Sr25519 and Ed25519)
    if let Ok(account_id) = sp_core::crypto::AccountId32::from_ss58check(addr_or_hex) {
        let bytes: &[u8; 32] = account_id.as_ref();
        return Ok(*bytes);
    }

    // Fall back to hex decoding
    let hex_str = addr_or_hex.strip_prefix("0x").unwrap_or(addr_or_hex);
    let bytes = hex::decode(hex_str)
        .map_err(|e| anyhow::anyhow!("Invalid receiver address (not valid SS58 or hex): {}", e))?;

    if bytes.len() != 32 {
        return Err(anyhow::anyhow!(
            "Invalid receiver public key: expected 32 bytes, got {}",
            bytes.len()
        ));
    }

    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(array)
}

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

        /// Cryptographic scheme for decryption (sr25519, ed25519)
        #[arg(long, default_value = "sr25519", value_parser = clap::value_parser!(libcps::crypto::CryptoScheme))]
        scheme: libcps::crypto::CryptoScheme,
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

        /// Receiver public key or SS58 address for encryption. If provided, data will be encrypted.
        /// Supports both SS58 addresses and hex-encoded public keys.
        #[arg(long)]
        receiver_public: Option<String>,

        /// Encryption algorithm (xchacha20, aesgcm256, chacha20)
        #[arg(long, default_value = "xchacha20")]
        cipher: String,

        /// Cryptographic scheme for encryption (sr25519, ed25519)
        #[arg(long, default_value = "sr25519", value_parser = clap::value_parser!(libcps::crypto::CryptoScheme))]
        scheme: libcps::crypto::CryptoScheme,
    },

    /// Update node metadata
    SetMeta {
        /// Node ID
        node_id: u64,

        /// New metadata
        data: String,

        /// Receiver public key or SS58 address for encryption. If provided, data will be encrypted.
        /// Supports both SS58 addresses and hex-encoded public keys.
        #[arg(long)]
        receiver_public: Option<String>,

        /// Encryption algorithm (xchacha20, aesgcm256, chacha20)
        #[arg(long, default_value = "xchacha20")]
        cipher: String,

        /// Cryptographic scheme for encryption (sr25519, ed25519)
        #[arg(long, default_value = "sr25519", value_parser = clap::value_parser!(libcps::crypto::CryptoScheme))]
        scheme: libcps::crypto::CryptoScheme,
    },

    /// Update node payload
    SetPayload {
        /// Node ID
        node_id: u64,

        /// New payload
        data: String,

        /// Receiver public key or SS58 address for encryption. If provided, data will be encrypted.
        /// Supports both SS58 addresses and hex-encoded public keys.
        #[arg(long)]
        receiver_public: Option<String>,

        /// Encryption algorithm (xchacha20, aesgcm256, chacha20)
        #[arg(long, default_value = "xchacha20")]
        cipher: String,

        /// Cryptographic scheme for encryption (sr25519, ed25519)
        #[arg(long, default_value = "sr25519", value_parser = clap::value_parser!(libcps::crypto::CryptoScheme))]
        scheme: libcps::crypto::CryptoScheme,
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

        /// Receiver public key or SS58 address for encryption. If provided, messages will be encrypted.
        /// Supports both SS58 addresses and hex-encoded public keys.
        #[arg(long)]
        receiver_public: Option<String>,

        /// Encryption algorithm (xchacha20, aesgcm256, chacha20)
        #[arg(long, default_value = "xchacha20")]
        cipher: String,

        /// Cryptographic scheme for encryption (sr25519, ed25519)
        #[arg(long, default_value = "sr25519", value_parser = clap::value_parser!(libcps::crypto::CryptoScheme))]
        scheme: libcps::crypto::CryptoScheme,
    },

    /// Publish node payload changes to MQTT topic
    Publish {
        /// MQTT topic to publish to
        topic: String,

        /// Node ID to monitor
        node_id: u64,

        /// (Deprecated) Polling interval in seconds - now monitors each block instead
        #[arg(long, default_value = "5")]
        interval: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Create blockchain config (crypto-free)
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
        Commands::Show {
            node_id,
            decrypt,
            scheme,
        } => {
            // Create cipher if decryption is requested
            let cipher = if decrypt {
                let suri = cli
                    .suri
                    .ok_or_else(|| anyhow::anyhow!("SURI required for decryption"))?;
                Some(libcps::crypto::Cipher::new(
                    suri,
                    libcps::crypto::EncryptionAlgorithm::XChaCha20Poly1305, // Placeholder; actual algorithm auto-detected in Cipher::decrypt
                    scheme,
                )?)
            } else {
                None
            };
            commands::show::execute(&blockchain_config, cipher.as_ref(), node_id, decrypt).await?;
        }
        Commands::Create {
            parent,
            meta,
            payload,
            receiver_public,
            cipher,
            scheme,
        } => {
            // Parse receiver public key if provided (supports both SS58 address and hex)
            let receiver_pub_bytes = if let Some(ref addr_or_hex) = receiver_public {
                Some(parse_receiver_public_key(addr_or_hex)?)
            } else {
                None
            };

            // Create cipher if encryption is requested
            let cipher = if receiver_public.is_some() {
                let algorithm = libcps::crypto::EncryptionAlgorithm::from_str(&cipher)
                    .map_err(|e| anyhow::anyhow!("Invalid cipher: {}", e))?;
                let suri = cli
                    .suri
                    .ok_or_else(|| anyhow::anyhow!("SURI required for encryption"))?;
                Some(libcps::crypto::Cipher::new(suri, algorithm, scheme)?)
            } else {
                None
            };
            commands::create::execute(
                &blockchain_config,
                cipher.as_ref(),
                parent,
                meta,
                payload,
                receiver_pub_bytes,
            )
            .await?;
        }
        Commands::SetMeta {
            node_id,
            data,
            receiver_public,
            cipher,
            scheme,
        } => {
            // Parse receiver public key if provided (supports both SS58 address and hex)
            let receiver_pub_bytes = if let Some(ref addr_or_hex) = receiver_public {
                Some(parse_receiver_public_key(addr_or_hex)?)
            } else {
                None
            };

            // Create cipher if encryption is requested
            let cipher = if receiver_public.is_some() {
                let algorithm = libcps::crypto::EncryptionAlgorithm::from_str(&cipher)
                    .map_err(|e| anyhow::anyhow!("Invalid cipher: {}", e))?;
                let suri = cli
                    .suri
                    .ok_or_else(|| anyhow::anyhow!("SURI required for encryption"))?;
                Some(libcps::crypto::Cipher::new(suri, algorithm, scheme)?)
            } else {
                None
            };
            commands::set_meta::execute(
                &blockchain_config,
                cipher.as_ref(),
                node_id,
                data,
                receiver_pub_bytes,
            )
            .await?;
        }
        Commands::SetPayload {
            node_id,
            data,
            receiver_public,
            cipher,
            scheme,
        } => {
            // Parse receiver public key if provided (supports both SS58 address and hex)
            let receiver_pub_bytes = if let Some(ref addr_or_hex) = receiver_public {
                Some(parse_receiver_public_key(addr_or_hex)?)
            } else {
                None
            };

            // Create cipher if encryption is requested
            let cipher = if receiver_public.is_some() {
                let algorithm = libcps::crypto::EncryptionAlgorithm::from_str(&cipher)
                    .map_err(|e| anyhow::anyhow!("Invalid cipher: {}", e))?;
                let suri = cli
                    .suri
                    .ok_or_else(|| anyhow::anyhow!("SURI required for encryption"))?;
                Some(libcps::crypto::Cipher::new(suri, algorithm, scheme)?)
            } else {
                None
            };
            commands::set_payload::execute(
                &blockchain_config,
                cipher.as_ref(),
                node_id,
                data,
                receiver_pub_bytes,
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
                receiver_public,
                cipher,
                scheme,
            } => {
                // Parse receiver public key if provided (supports both SS58 address and hex)
                let receiver_pub_bytes = if let Some(ref addr_or_hex) = receiver_public {
                    Some(parse_receiver_public_key(addr_or_hex)?)
                } else {
                    None
                };

                // Create cipher if encryption is requested
                let cipher = if receiver_public.is_some() {
                    let algorithm = libcps::crypto::EncryptionAlgorithm::from_str(&cipher)
                        .map_err(|e| anyhow::anyhow!("Invalid cipher: {}", e))?;
                    let suri = cli
                        .suri
                        .ok_or_else(|| anyhow::anyhow!("SURI required for encryption"))?;
                    Some(libcps::crypto::Cipher::new(suri, algorithm, scheme)?)
                } else {
                    None
                };
                commands::mqtt::subscribe(
                    &blockchain_config,
                    cipher.as_ref(),
                    &mqtt_config,
                    &topic,
                    node_id,
                    receiver_pub_bytes,
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
