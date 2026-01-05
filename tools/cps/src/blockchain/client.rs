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
//! Blockchain client and connection management.
//!
//! This module provides the [`Client`] type for connecting to a Robonomics blockchain
//! node and managing account keypairs for transaction signing.
//!
//! # Examples
//!
//! ```no_run
//! use libcps::blockchain::{Client, Config};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config {
//!         ws_url: "ws://localhost:9944".to_string(),
//!         suri: Some("//Alice".to_string()),
//!     };
//!     
//!     let client = Client::new(&config).await?;
//!     let keypair = client.require_keypair()?;
//!     
//!     println!("Connected with account: {:?}", keypair.public_key());
//!     Ok(())
//! }
//! ```

use anyhow::{anyhow, Result};
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::{sr25519::Keypair, SecretUri};
use sp_core::Pair;

/// Configuration for blockchain connection and encryption.
///
/// # Fields
///
/// * `ws_url` - WebSocket URL of the blockchain node (e.g., "ws://localhost:9944")
/// * `suri` - Optional secret URI for account (e.g., "//Alice" or a seed phrase)
/// * `algorithm` - Encryption algorithm to use (set once on node launch)
/// * `scheme` - Cryptographic scheme to use (sr25519 or ed25519, set once on node launch)
#[derive(Clone)]
pub struct Config {
    /// WebSocket URL of the blockchain node
    pub ws_url: String,
    /// Optional secret URI for signing transactions
    pub suri: Option<String>,
    /// Encryption algorithm (set once on node launch)
    pub algorithm: crate::crypto::EncryptionAlgorithm,
    /// Cryptographic scheme (set once on node launch)
    pub scheme: crate::crypto::CryptoScheme,
}

impl Config {
    /// Encrypt data using the configured algorithm and scheme.
    ///
    /// # Arguments
    ///
    /// * `plaintext` - The data to encrypt
    /// * `receiver_public` - The recipient's public key for encryption
    ///
    /// # Returns
    ///
    /// Returns encrypted bytes in JSON format
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - SURI is not configured
    /// - Keypair parsing fails
    /// - Encryption fails
    pub fn encrypt(&self, plaintext: &[u8], receiver_public: &[u8]) -> Result<Vec<u8>> {
        let suri = self
            .suri
            .as_ref()
            .ok_or_else(|| anyhow!("SURI required for encryption"))?;

        match self.scheme {
            crate::crypto::CryptoScheme::Sr25519 => {
                let pair = sp_core::sr25519::Pair::from_string(suri, None)
                    .map_err(|e| anyhow!("Failed to parse SR25519 keypair: {:?}", e))?;
                
                // Parse receiver public key
                if receiver_public.len() != 32 {
                    return Err(anyhow!("Invalid receiver public key length"));
                }
                let mut public_bytes = [0u8; 32];
                public_bytes.copy_from_slice(receiver_public);
                let receiver = sp_core::sr25519::Public::from_raw(public_bytes);
                
                crate::crypto::encrypt(plaintext, &pair, &receiver, self.algorithm)
            }
            crate::crypto::CryptoScheme::Ed25519 => {
                let pair = sp_core::ed25519::Pair::from_string(suri, None)
                    .map_err(|e| anyhow!("Failed to parse ED25519 keypair: {:?}", e))?;
                
                // Parse receiver public key
                if receiver_public.len() != 32 {
                    return Err(anyhow!("Invalid receiver public key length"));
                }
                let mut public_bytes = [0u8; 32];
                public_bytes.copy_from_slice(receiver_public);
                let receiver = sp_core::ed25519::Public::from_raw(public_bytes);
                
                crate::crypto::encrypt(plaintext, &pair, &receiver, self.algorithm)
            }
        }
    }

    /// Decrypt data using the configured scheme.
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - JSON-formatted encrypted data
    /// * `expected_sender` - Optional sender public key for verification
    ///
    /// # Returns
    ///
    /// Returns decrypted plaintext bytes
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - SURI is not configured
    /// - Keypair parsing fails
    /// - Decryption fails
    /// - Sender verification fails (if expected_sender provided)
    pub fn decrypt(&self, ciphertext: &[u8], expected_sender: Option<&[u8]>) -> Result<Vec<u8>> {
        let suri = self
            .suri
            .as_ref()
            .ok_or_else(|| anyhow!("SURI required for decryption"))?;

        match self.scheme {
            crate::crypto::CryptoScheme::Sr25519 => {
                let pair = sp_core::sr25519::Pair::from_string(suri, None)
                    .map_err(|e| anyhow!("Failed to parse SR25519 keypair: {:?}", e))?;

                // Convert expected_sender to proper type if provided
                let expected_public = if let Some(sender_bytes) = expected_sender {
                    if sender_bytes.len() != 32 {
                        return Err(anyhow!("Invalid sender public key length"));
                    }
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(sender_bytes);
                    Some(sp_core::sr25519::Public::from_raw(arr))
                } else {
                    None
                };

                crate::crypto::decrypt(ciphertext, &pair, expected_public.as_ref())
            }
            crate::crypto::CryptoScheme::Ed25519 => {
                let pair = sp_core::ed25519::Pair::from_string(suri, None)
                    .map_err(|e| anyhow!("Failed to parse ED25519 keypair: {:?}", e))?;

                // Convert expected_sender to proper type if provided
                let expected_public = if let Some(sender_bytes) = expected_sender {
                    if sender_bytes.len() != 32 {
                        return Err(anyhow!("Invalid sender public key length"));
                    }
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(sender_bytes);
                    Some(sp_core::ed25519::Public::from_raw(arr))
                } else {
                    None
                };

                crate::crypto::decrypt(ciphertext, &pair, expected_public.as_ref())
            }
        }
    }
}

/// Blockchain client for interacting with Robonomics CPS pallet.
///
/// This client manages the connection to a Substrate-based blockchain node
/// and provides access to the subxt API client and optional signing keypair.
///
/// # Examples
///
/// ```no_run
/// use libcps::blockchain::{Client, Config};
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let config = Config {
///     ws_url: "ws://localhost:9944".to_string(),
///     suri: None, // Read-only access
/// };
///
/// let client = Client::new(&config).await?;
/// // Use client.api to query blockchain state
/// # Ok(())
/// # }
/// ```
pub struct Client {
    /// Subxt client for blockchain interaction
    pub api: OnlineClient<PolkadotConfig>,
    /// Optional keypair for signing transactions
    pub keypair: Option<Keypair>,
}

impl Client {
    /// Create a new blockchain client.
    ///
    /// Connects to the specified WebSocket URL and optionally loads a keypair
    /// from the provided SURI.
    ///
    /// # Arguments
    ///
    /// * `config` - Connection configuration
    ///
    /// # Returns
    ///
    /// A `Result` containing the client or an error if connection fails.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Cannot connect to the blockchain node
    /// - Cannot parse the SURI
    /// - Cannot derive the keypair from SURI
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libcps::blockchain::{Client, Config};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let config = Config {
    ///     ws_url: "ws://localhost:9944".to_string(),
    ///     suri: Some("//Alice".to_string()),
    /// };
    ///
    /// let client = Client::new(&config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(config: &Config) -> Result<Self> {
        // Connect to the blockchain
        let api = OnlineClient::<PolkadotConfig>::from_url(&config.ws_url)
            .await
            .map_err(|e| anyhow!("Failed to connect to {}: {}", config.ws_url, e))?;

        // Parse keypair if SURI provided
        let keypair = if let Some(suri) = &config.suri {
            let uri: SecretUri = suri
                .parse()
                .map_err(|e| anyhow!("Failed to parse SURI: {e}"))?;
            Some(Keypair::from_uri(&uri).map_err(|e| anyhow!("Failed to create keypair: {e}"))?)
        } else {
            None
        };

        Ok(Self { api, keypair })
    }

    /// Get the keypair, returning an error if not available.
    ///
    /// This is useful for operations that require signing transactions.
    ///
    /// # Returns
    ///
    /// A reference to the keypair or an error if no keypair was loaded.
    ///
    /// # Errors
    ///
    /// Returns an error if no SURI was provided during client creation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libcps::blockchain::{Client, Config};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let config = Config {
    ///     ws_url: "ws://localhost:9944".to_string(),
    ///     suri: Some("//Alice".to_string()),
    /// };
    ///
    /// let client = Client::new(&config).await?;
    /// let keypair = client.require_keypair()?;
    /// // Use keypair to sign transactions
    /// # Ok(())
    /// # }
    /// ```
    pub fn require_keypair(&self) -> Result<&Keypair> {
        self.keypair
            .as_ref()
            .ok_or_else(|| anyhow!("This operation requires an account. Please provide --suri or set ROBONOMICS_SURI environment variable."))
    }
}
