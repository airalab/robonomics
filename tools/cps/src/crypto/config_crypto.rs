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
//! Configuration-based encryption/decryption implementation.
//!
//! This module provides a generic approach to encryption and decryption
//! using configuration objects that contain SURI strings, implementing
//! the Encrypt and Decrypt traits.

use crate::blockchain::Config;
use crate::crypto::{decrypt, encrypt, CryptoScheme, Decrypt, Encrypt, EncryptionAlgorithm};
use anyhow::{anyhow, Result};
use sp_core::Pair;

/// Implements Encrypt trait for blockchain Config.
///
/// This allows encrypting data using the keypair derived from
/// the SURI in the configuration.
impl Encrypt for Config {
    fn encrypt(
        &self,
        plaintext: &[u8],
        algorithm: EncryptionAlgorithm,
        scheme: CryptoScheme,
    ) -> Result<Vec<u8>> {
        let suri = self
            .suri
            .as_ref()
            .ok_or_else(|| anyhow!("SURI required for encryption"))?;

        match scheme {
            CryptoScheme::Sr25519 => {
                let pair = sp_core::sr25519::Pair::from_string(suri, None)
                    .map_err(|e| anyhow!("Failed to parse SR25519 keypair: {:?}", e))?;
                // Encrypt to self (for storage)
                let public = pair.public();
                encrypt(plaintext, &pair, &public, algorithm)
            }
            CryptoScheme::Ed25519 => {
                let pair = sp_core::ed25519::Pair::from_string(suri, None)
                    .map_err(|e| anyhow!("Failed to parse ED25519 keypair: {:?}", e))?;
                // Encrypt to self (for storage)
                let public = pair.public();
                encrypt(plaintext, &pair, &public, algorithm)
            }
        }
    }
}

/// Implements Decrypt trait for blockchain Config.
///
/// This allows decrypting data using the keypair derived from
/// the SURI in the configuration.
impl Decrypt for Config {
    fn decrypt(
        &self,
        ciphertext: &[u8],
        scheme: CryptoScheme,
        expected_sender: Option<&[u8]>,
    ) -> Result<Vec<u8>> {
        let suri = self
            .suri
            .as_ref()
            .ok_or_else(|| anyhow!("SURI required for decryption"))?;

        match scheme {
            CryptoScheme::Sr25519 => {
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

                decrypt(ciphertext, &pair, expected_public.as_ref())
            }
            CryptoScheme::Ed25519 => {
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

                decrypt(ciphertext, &pair, expected_public.as_ref())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip_sr25519() {
        let config = Config {
            ws_url: "ws://localhost:9944".to_string(),
            suri: Some("//Alice".to_string()),
        };

        let plaintext = b"Hello, World!";
        let algorithm = EncryptionAlgorithm::XChaCha20Poly1305;
        let scheme = CryptoScheme::Sr25519;

        let encrypted = config.encrypt(plaintext, algorithm, scheme).unwrap();
        let decrypted = config.decrypt(&encrypted, scheme, None).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_ed25519() {
        let config = Config {
            ws_url: "ws://localhost:9944".to_string(),
            suri: Some("//Alice".to_string()),
        };

        let plaintext = b"Hello, World!";
        let algorithm = EncryptionAlgorithm::AesGcm256;
        let scheme = CryptoScheme::Ed25519;

        let encrypted = config.encrypt(plaintext, algorithm, scheme).unwrap();
        let decrypted = config.decrypt(&encrypted, scheme, None).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypt_without_suri_fails() {
        let config = Config {
            ws_url: "ws://localhost:9944".to_string(),
            suri: None,
        };

        let plaintext = b"Hello, World!";
        let algorithm = EncryptionAlgorithm::XChaCha20Poly1305;
        let scheme = CryptoScheme::Sr25519;

        assert!(config.encrypt(plaintext, algorithm, scheme).is_err());
    }

    #[test]
    fn test_decrypt_without_suri_fails() {
        let config = Config {
            ws_url: "ws://localhost:9944".to_string(),
            suri: None,
        };

        let ciphertext = b"fake encrypted data";
        let scheme = CryptoScheme::Sr25519;

        assert!(config.decrypt(ciphertext, scheme, None).is_err());
    }
}
