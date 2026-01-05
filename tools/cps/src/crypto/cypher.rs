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
//! Cypher configuration and encryption/decryption operations.
//!
//! This module provides the `Cypher` struct which encapsulates cryptographic
//! configuration and operations, separating crypto concerns from blockchain config.

use anyhow::{anyhow, Result};
use sp_core::Pair;

/// Cypher configuration for encryption and decryption operations.
///
/// This struct holds the cryptographic configuration including the secret key,
/// algorithm, and scheme. It provides methods to encrypt and decrypt data.
///
/// # Fields
///
/// * `suri` - Secret URI for the keypair (required for crypto operations)
/// * `algorithm` - The AEAD encryption algorithm to use
/// * `scheme` - The cryptographic scheme (SR25519 or ED25519)
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::{Cypher, EncryptionAlgorithm, CryptoScheme};
///
/// let cypher = Cypher::new(
///     "//Alice".to_string(),
///     EncryptionAlgorithm::XChaCha20Poly1305,
///     CryptoScheme::Sr25519,
/// );
///
/// let plaintext = b"secret message";
/// let receiver_public = &[0u8; 32]; // receiver's public key
/// let encrypted = cypher.encrypt(plaintext, receiver_public).unwrap();
/// let decrypted = cypher.decrypt(&encrypted, None).unwrap();
/// ```
#[derive(Clone)]
pub struct Cypher {
    /// Secret URI for the keypair (required)
    suri: String,
    /// Encryption algorithm
    algorithm: crate::crypto::EncryptionAlgorithm,
    /// Cryptographic scheme
    scheme: crate::crypto::CryptoScheme,
}

impl Cypher {
    /// Create a new Cypher configuration.
    ///
    /// # Arguments
    ///
    /// * `suri` - Secret URI for the keypair
    /// * `algorithm` - Encryption algorithm to use
    /// * `scheme` - Cryptographic scheme to use
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::crypto::{Cypher, EncryptionAlgorithm, CryptoScheme};
    ///
    /// let cypher = Cypher::new(
    ///     "//Alice".to_string(),
    ///     EncryptionAlgorithm::XChaCha20Poly1305,
    ///     CryptoScheme::Sr25519,
    /// );
    /// ```
    pub fn new(
        suri: String,
        algorithm: crate::crypto::EncryptionAlgorithm,
        scheme: crate::crypto::CryptoScheme,
    ) -> Self {
        Self {
            suri,
            algorithm,
            scheme,
        }
    }

    /// Get the encryption algorithm.
    pub fn algorithm(&self) -> crate::crypto::EncryptionAlgorithm {
        self.algorithm
    }

    /// Get the cryptographic scheme.
    pub fn scheme(&self) -> crate::crypto::CryptoScheme {
        self.scheme
    }

    /// Encrypt data for a specific receiver.
    ///
    /// # Arguments
    ///
    /// * `plaintext` - The data to encrypt
    /// * `receiver_public` - The recipient's public key (32 bytes)
    ///
    /// # Returns
    ///
    /// Returns encrypted bytes in JSON format containing:
    /// - version: Format version
    /// - algorithm: Algorithm identifier
    /// - from: Sender's public key (base58)
    /// - nonce: Base64-encoded nonce
    /// - ciphertext: Base64-encoded encrypted data
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Keypair parsing fails
    /// - Receiver public key is invalid
    /// - Encryption fails
    pub fn encrypt(&self, plaintext: &[u8], receiver_public: &[u8]) -> Result<Vec<u8>> {
        // Validate receiver public key length
        if receiver_public.len() != 32 {
            return Err(anyhow!("Invalid receiver public key length: expected 32 bytes, got {}", receiver_public.len()));
        }

        match self.scheme {
            crate::crypto::CryptoScheme::Sr25519 => {
                let pair = sp_core::sr25519::Pair::from_string(&self.suri, None)
                    .map_err(|e| anyhow!("Failed to parse SR25519 keypair: {:?}", e))?;
                
                // Parse receiver public key
                let mut public_bytes = [0u8; 32];
                public_bytes.copy_from_slice(receiver_public);
                let receiver = sp_core::sr25519::Public::from_raw(public_bytes);
                
                // Call the existing encrypt function from encryption module
                super::encryption::encrypt(plaintext, &pair, &receiver, self.algorithm)
            }
            crate::crypto::CryptoScheme::Ed25519 => {
                let pair = sp_core::ed25519::Pair::from_string(&self.suri, None)
                    .map_err(|e| anyhow!("Failed to parse ED25519 keypair: {:?}", e))?;
                
                // Parse receiver public key
                let mut public_bytes = [0u8; 32];
                public_bytes.copy_from_slice(receiver_public);
                let receiver = sp_core::ed25519::Public::from_raw(public_bytes);
                
                // Call the existing encrypt function from encryption module
                super::encryption::encrypt(plaintext, &pair, &receiver, self.algorithm)
            }
        }
    }

    /// Decrypt data.
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - JSON-formatted encrypted data
    /// * `expected_sender` - Optional sender public key for verification (32 bytes)
    ///
    /// # Returns
    ///
    /// Returns decrypted plaintext bytes
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Keypair parsing fails
    /// - Expected sender public key is invalid
    /// - Decryption fails
    /// - Sender verification fails (if expected_sender provided)
    pub fn decrypt(&self, ciphertext: &[u8], expected_sender: Option<&[u8]>) -> Result<Vec<u8>> {
        match self.scheme {
            crate::crypto::CryptoScheme::Sr25519 => {
                let pair = sp_core::sr25519::Pair::from_string(&self.suri, None)
                    .map_err(|e| anyhow!("Failed to parse SR25519 keypair: {:?}", e))?;

                // Convert expected_sender to proper type if provided
                let expected_public = if let Some(sender_bytes) = expected_sender {
                    if sender_bytes.len() != 32 {
                        return Err(anyhow!("Invalid sender public key length: expected 32 bytes, got {}", sender_bytes.len()));
                    }
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(sender_bytes);
                    Some(sp_core::sr25519::Public::from_raw(arr))
                } else {
                    None
                };

                // Call the existing decrypt function from encryption module
                super::encryption::decrypt(ciphertext, &pair, expected_public.as_ref())
            }
            crate::crypto::CryptoScheme::Ed25519 => {
                let pair = sp_core::ed25519::Pair::from_string(&self.suri, None)
                    .map_err(|e| anyhow!("Failed to parse ED25519 keypair: {:?}", e))?;

                // Convert expected_sender to proper type if provided
                let expected_public = if let Some(sender_bytes) = expected_sender {
                    if sender_bytes.len() != 32 {
                        return Err(anyhow!("Invalid sender public key length: expected 32 bytes, got {}", sender_bytes.len()));
                    }
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(sender_bytes);
                    Some(sp_core::ed25519::Public::from_raw(arr))
                } else {
                    None
                };

                // Call the existing decrypt function from encryption module
                super::encryption::decrypt(ciphertext, &pair, expected_public.as_ref())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cypher_creation() {
        let cypher = Cypher::new(
            "//Alice".to_string(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
            crate::crypto::CryptoScheme::Sr25519,
        );
        
        assert_eq!(cypher.algorithm(), crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305);
        assert_eq!(cypher.scheme(), crate::crypto::CryptoScheme::Sr25519);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_sr25519() {
        let cypher = Cypher::new(
            "//Alice".to_string(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
            crate::crypto::CryptoScheme::Sr25519,
        );

        // Get Alice's public key for self-encryption
        let pair = sp_core::sr25519::Pair::from_string("//Alice", None).unwrap();
        let public_key = pair.public().0;

        let plaintext = b"Hello, World!";
        let encrypted = cypher.encrypt(plaintext, &public_key).unwrap();
        let decrypted = cypher.decrypt(&encrypted, None).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_ed25519() {
        let cypher = Cypher::new(
            "//Alice".to_string(),
            crate::crypto::EncryptionAlgorithm::AesGcm256,
            crate::crypto::CryptoScheme::Ed25519,
        );

        // Get Alice's public key for self-encryption
        let pair = sp_core::ed25519::Pair::from_string("//Alice", None).unwrap();
        let public_key = pair.public().0;

        let plaintext = b"Hello, World!";
        let encrypted = cypher.encrypt(plaintext, &public_key).unwrap();
        let decrypted = cypher.decrypt(&encrypted, None).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_invalid_receiver_public_key_length() {
        let cypher = Cypher::new(
            "//Alice".to_string(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
            crate::crypto::CryptoScheme::Sr25519,
        );

        let plaintext = b"test";
        let invalid_key = &[0u8; 16]; // Wrong length
        
        assert!(cypher.encrypt(plaintext, invalid_key).is_err());
    }
}
