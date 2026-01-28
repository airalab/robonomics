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

/// Parses a receiver public key from either an SS58 address or a hex-encoded 32-byte key.
///
/// # Supported formats
/// - **SS58 address**: A valid Substrate SS58-encoded account ID. Decoding is attempted first
///   using `sp_core::crypto::AccountId32::from_ss58check`, which supports both Sr25519 and
///   Ed25519 (they share the same 32-byte public key length).
/// - **Hex string**: A 64-hex-character string representing a 32-byte public key. An optional
///   `0x` prefix is allowed (e.g. `0xdeadbeef...` or `deadbeef...`).
///
/// # Conversion process
/// 1. Try to decode `addr_or_hex` as an SS58 address. On success, the underlying 32-byte
///    account ID is returned.
/// 2. If SS58 decoding fails, strip a leading `0x` (if present) and attempt to decode the
///    remaining string as hex.
///
/// # Errors
/// - Returns an error if the value is neither a valid SS58 address nor a valid hex string.
/// - Returns an error if the hex decoding succeeds but the resulting byte length is not
///   exactly 32 bytes.
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
#[command(version, about = "CPS CLI - Robonomics Cyber-Physical Systems", long_about = None)]
struct Cli {
    /// WebSocket URL for blockchain connection
    #[arg(long, env = "ROBONOMICS_WS_URL", default_value = "ws://localhost:9944")]
    ws_url: String,

    /// Account secret URI (e.g., //Alice, //Bob, or seed phrase)
    #[arg(long, env = "ROBONOMICS_SURI")]
    suri: Option<String>,

    #[cfg(feature = "mqtt")]
    /// MQTT broker URL
    #[arg(
        long,
        env = "ROBONOMICS_MQTT_BROKER",
        default_value = "mqtt://localhost:1883"
    )]
    mqtt_broker: String,

    #[cfg(feature = "mqtt")]
    /// MQTT username
    #[arg(long, env = "ROBONOMICS_MQTT_USERNAME")]
    mqtt_username: Option<String>,

    #[cfg(feature = "mqtt")]
    /// MQTT password
    #[arg(long, env = "ROBONOMICS_MQTT_PASSWORD")]
    mqtt_password: Option<String>,

    #[cfg(feature = "mqtt")]
    /// MQTT client ID
    #[arg(long, env = "ROBONOMICS_MQTT_CLIENT_ID")]
    mqtt_client_id: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Display node information and its children in a beautiful tree format
    #[command(
        long_about = "Display node information and its children in a beautiful tree format.

EXAMPLES:
    # Show node 0
    cps show 0

    # Show node with decryption attempt (using SR25519)
    cps show 5 --decrypt

    # Show node with ED25519 decryption
    cps show 5 --decrypt --scheme ed25519"
    )]
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
    #[command(long_about = "Create a new node (root or child).

EXAMPLES:
    # Create root node
    cps create --meta '{\"type\":\"sensor\"}' --payload '22.5C'

    # Create child node
    cps create --parent 0 --payload 'operational data'

    # Create with encryption (SR25519, default)
    cps create --parent 0 --payload 'secret data' \\
        --receiver-public 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY

    # Create with ED25519 encryption (Home Assistant compatible)
    cps create --parent 0 --payload 'secret data' \\
        --receiver-public 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY \\
        --scheme ed25519

    # Create with specific cipher
    cps create --parent 0 --payload 'secret data' \\
        --receiver-public 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY \\
        --cipher aesgcm256")]
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
    #[command(long_about = "Update node metadata.

EXAMPLES:
    # Update metadata
    cps set-meta 5 '{\"name\":\"Updated Sensor\"}'

    # Update with encryption
    cps set-meta 5 'private config' \\
        --receiver-public 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY

    # Update with ED25519 encryption
    cps set-meta 5 'private config' \\
        --receiver-public 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY \\
        --scheme ed25519")]
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
    #[command(long_about = "Update node payload (operational data).

EXAMPLES:
    # Update temperature reading
    cps set-payload 5 '23.1C'

    # Update with encryption
    cps set-payload 5 'encrypted telemetry' \\
        --receiver-public 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY

    # Update with ED25519 and AES-GCM
    cps set-payload 5 'encrypted telemetry' \\
        --receiver-public 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY \\
        --scheme ed25519 --cipher aesgcm256")]
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
    #[command(long_about = "Move a node to a new parent.

EXAMPLES:
    # Move node 5 under node 3
    cps move 5 3

FEATURES:
    - Automatic cycle detection (prevents moving a node under its own descendant)
    - Path validation")]
    Move {
        /// Node ID to move
        node_id: u64,

        /// New parent node ID
        new_parent_id: u64,
    },

    /// Delete a node (must have no children)
    #[command(long_about = "Delete a node (must have no children).

EXAMPLES:
    # Remove node with confirmation
    cps remove 5

    # Remove without confirmation
    cps remove 5 --force")]
    Remove {
        /// Node ID to remove
        node_id: u64,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// MQTT bridge commands
    #[cfg(feature = "mqtt")]
    #[command(subcommand)]
    Mqtt(MqttCommands),
}

#[cfg(feature = "mqtt")]
#[derive(Subcommand)]
enum MqttCommands {
    /// Subscribe to MQTT topic and update node payload with received messages
    #[command(
        long_about = "Subscribe to MQTT topic and update node payload with received messages.

Connects to MQTT broker, subscribes to a topic, and updates the blockchain node payload 
with each received message. Supports real-time encryption for secure IoT integration.

EXAMPLES:
    # Subscribe to sensor data
    cps mqtt subscribe 'sensors/temp01' 5

    # Subscribe with encryption (SR25519)
    cps mqtt subscribe 'sensors/temp01' 5 \\
        --receiver-public 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY

    # Subscribe with ED25519 encryption (Home Assistant compatible)
    cps mqtt subscribe 'homeassistant/sensor/temp' 5 \\
        --receiver-public 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY \\
        --scheme ed25519

    # Subscribe with specific cipher
    cps mqtt subscribe 'sensors/temp01' 5 \\
        --receiver-public 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY \\
        --cipher aesgcm256

BEHAVIOR:
    - Connects to MQTT broker
    - Subscribes to specified topic
    - On each message: updates node payload on blockchain
    - Displays colorful logs with timestamps for each update
    - Auto-reconnects on connection failures"
    )]
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
    #[command(long_about = "Publish node payload changes to MQTT topic.

Monitors blockchain node for PayloadSet events and publishes payload changes to MQTT topic
in real-time. Event-driven approach ensures efficient operation without unnecessary queries.

EXAMPLES:
    # Publish node changes
    cps mqtt publish 'actuators/valve01' 10

    # With explicit broker configuration
    cps mqtt publish 'actuators/valve01' 10 \\
        --mqtt-broker mqtt://broker.local:1883 \\
        --mqtt-username user \\
        --mqtt-password pass

BEHAVIOR:
    - Subscribes to finalized blockchain blocks
    - Monitors PayloadSet events for the specified node
    - Only queries and publishes when payload actually changes
    - Automatically decrypts encrypted payloads
    - Displays colorful logs with timestamps and block numbers
    - Auto-reconnects on connection failures

TECHNICAL DETAILS:
    - Event-driven monitoring (no polling)
    - Real-time payload change detection
    - Graceful shutdown on exit")]
    Publish {
        /// MQTT topic to publish to
        topic: String,

        /// Node ID to monitor
        node_id: u64,
        
        /// Decrypt encrypted blockchain payloads before publishing to MQTT
        /// The encryption algorithm and scheme are auto-detected from the encrypted data
        #[arg(short = 'd', long)]
        decrypt: bool,
    },

    /// Start MQTT bridge from configuration file
    #[command(long_about = "Start MQTT bridge from configuration file.

Reads MQTT and blockchain configuration from a TOML file and starts all
configured subscribe and publish bridges concurrently.

EXAMPLES:
    # Start from config file
    cps mqtt start -c config.toml
    
    # With custom config path
    cps mqtt start --config /etc/cps/mqtt.toml

CONFIGURATION FILE FORMAT:
    See examples/mqtt_config.toml for a complete example.

BEHAVIOR:
    - Loads configuration from TOML file
    - Validates all settings
    - Spawns concurrent tasks for all bridges
    - Runs indefinitely until interrupted
    - Auto-reconnects on failures")]
    Start {
        /// Path to TOML configuration file
        #[arg(short = 'c', long)]
        config: String,
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

    #[cfg(feature = "mqtt")]
    // Create MQTT config
    let mqtt_config = mqtt::Config {
        broker: cli.mqtt_broker.clone(),
        username: cli.mqtt_username.clone(),
        password: cli.mqtt_password.clone(),
        client_id: cli.mqtt_client_id.clone(),
        blockchain: None,  // Not used for CLI commands
        subscribe: Vec::new(),  // Not used for CLI commands
        publish: Vec::new(),  // Not used for CLI commands
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

            // Encryption requires BOTH sender SURI and receiver public key.
            // - SURI (sender's seed phrase): Used to derive the sender's keypair for ECDH
            // - receiver_public: The recipient's public key for deriving the shared secret
            // If receiver_public is None, data will be stored as plaintext (no encryption).
            let (cipher_opt, algorithm_opt) = if receiver_public.is_some() {
                let algorithm = libcps::crypto::EncryptionAlgorithm::from_str(&cipher)
                    .map_err(|e| anyhow::anyhow!("Invalid cipher: {}", e))?;
                let suri = cli
                    .suri
                    .ok_or_else(|| anyhow::anyhow!("SURI required for encryption"))?;
                (Some(libcps::crypto::Cipher::new(suri, scheme)?), Some(algorithm))
            } else {
                (None, None)
            };
            commands::create::execute(
                &blockchain_config,
                cipher_opt.as_ref(),
                parent,
                meta,
                payload,
                receiver_pub_bytes,
                algorithm_opt,
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
            let (cipher_opt, algorithm_opt) = if receiver_public.is_some() {
                let algorithm = libcps::crypto::EncryptionAlgorithm::from_str(&cipher)
                    .map_err(|e| anyhow::anyhow!("Invalid cipher: {}", e))?;
                let suri = cli
                    .suri
                    .ok_or_else(|| anyhow::anyhow!("SURI required for encryption"))?;
                (Some(libcps::crypto::Cipher::new(suri, scheme)?), Some(algorithm))
            } else {
                (None, None)
            };
            commands::set_meta::execute(
                &blockchain_config,
                cipher_opt.as_ref(),
                node_id,
                data,
                receiver_pub_bytes,
                algorithm_opt,
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
            let (cipher_opt, algorithm_opt) = if receiver_public.is_some() {
                let algorithm = libcps::crypto::EncryptionAlgorithm::from_str(&cipher)
                    .map_err(|e| anyhow::anyhow!("Invalid cipher: {}", e))?;
                let suri = cli
                    .suri
                    .ok_or_else(|| anyhow::anyhow!("SURI required for encryption"))?;
                (Some(libcps::crypto::Cipher::new(suri, scheme)?), Some(algorithm))
            } else {
                (None, None)
            };
            commands::set_payload::execute(
                &blockchain_config,
                cipher_opt.as_ref(),
                node_id,
                data,
                receiver_pub_bytes,
                algorithm_opt,
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
        #[cfg(feature = "mqtt")]
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
                let (cipher_opt, algorithm_opt) = if receiver_public.is_some() {
                    let algorithm = libcps::crypto::EncryptionAlgorithm::from_str(&cipher)
                        .map_err(|e| anyhow::anyhow!("Invalid cipher: {}", e))?;
                    let suri = cli
                        .suri
                        .ok_or_else(|| anyhow::anyhow!("SURI required for encryption"))?;
                    (Some(libcps::crypto::Cipher::new(suri, scheme)?), Some(algorithm))
                } else {
                    (None, None)
                };
                commands::mqtt::subscribe(
                    &blockchain_config,
                    cipher_opt.as_ref(),
                    &mqtt_config,
                    &topic,
                    node_id,
                    receiver_pub_bytes,
                    algorithm_opt,
                )
                .await?;
            }
            MqttCommands::Publish { topic, node_id, decrypt } => {
                commands::mqtt::publish(&blockchain_config, &mqtt_config, &topic, node_id, decrypt).await?;
            }
            MqttCommands::Start { config } => {
                // Load config from file and start all bridges
                display::tree::progress(&format!("Loading configuration from {}...", config));
                let mqtt_config = mqtt::Config::from_file(&config)?;
                display::tree::success("Configuration loaded successfully");
                
                // Validate that blockchain config is present
                if mqtt_config.blockchain.is_none() {
                    return Err(anyhow::anyhow!(
                        "Configuration file must include [blockchain] section with ws_url"
                    ));
                }
                
                display::tree::info(&format!(
                    "Starting {} subscribe bridge(s) and {} publish bridge(s)...",
                    mqtt_config.subscribe.len(),
                    mqtt_config.publish.len()
                ));
                
                mqtt_config.start().await?;
            }
        },
    }

    Ok(())
}
