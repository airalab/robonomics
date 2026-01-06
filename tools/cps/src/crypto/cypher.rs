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
//! This module provides the `Cypher` enum which encapsulates cryptographic
//! configuration and operations, separating crypto concerns from blockchain config.

use anyhow::{anyhow, Result};
use sp_core::Pair;

/// Cypher configuration for encryption and decryption operations.
///
/// This enum holds the cryptographic keypair and algorithm. The keypair is
/// created once during construction to avoid repeated parsing overhead.
///
/// # Variants
///
/// * `Sr25519` - SR25519 keypair with encryption algorithm
/// * `Ed25519` - ED25519 keypair with encryption algorithm
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
/// ).unwrap();
///
/// let plaintext = b"secret message";
/// let receiver_public = &[0u8; 32]; // receiver's public key
/// let encrypted = cypher.encrypt(plaintext, receiver_public).unwrap();
/// let decrypted = cypher.decrypt(&encrypted, None).unwrap();
/// ```
pub enum Cypher {
    /// SR25519 cryptographic scheme
    Sr25519 {
        /// The keypair
        pair: sp_core::sr25519::Pair,
        /// Encryption algorithm
        algorithm: crate::crypto::EncryptionAlgorithm,
    },
    /// ED25519 cryptographic scheme
    Ed25519 {
        /// The keypair
        pair: sp_core::ed25519::Pair,
        /// Encryption algorithm
        algorithm: crate::crypto::EncryptionAlgorithm,
    },
}

impl Cypher {
    /// Create a new Cypher configuration.
    ///
    /// The keypair is created once during construction to avoid repeated parsing overhead.
    ///
    /// # Arguments
    ///
    /// * `suri` - Secret URI for the keypair
    /// * `algorithm` - Encryption algorithm to use
    /// * `scheme` - Cryptographic scheme to use
    ///
    /// # Returns
    ///
    /// Returns a Cypher instance with the parsed keypair
    ///
    /// # Errors
    ///
    /// Returns error if keypair parsing fails
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
    /// ).unwrap();
    /// ```
    pub fn new(
        suri: String,
        algorithm: crate::crypto::EncryptionAlgorithm,
        scheme: crate::crypto::CryptoScheme,
    ) -> Result<Self> {
        match scheme {
            crate::crypto::CryptoScheme::Sr25519 => {
                let pair = sp_core::sr25519::Pair::from_string(&suri, None)
                    .map_err(|e| anyhow!("Failed to parse SR25519 keypair: {:?}", e))?;
                Ok(Cypher::Sr25519 { pair, algorithm })
            }
            crate::crypto::CryptoScheme::Ed25519 => {
                let pair = sp_core::ed25519::Pair::from_string(&suri, None)
                    .map_err(|e| anyhow!("Failed to parse ED25519 keypair: {:?}", e))?;
                Ok(Cypher::Ed25519 { pair, algorithm })
            }
        }
    }

    /// Get the encryption algorithm.
    pub fn algorithm(&self) -> crate::crypto::EncryptionAlgorithm {
        match self {
            Cypher::Sr25519 { algorithm, .. } => *algorithm,
            Cypher::Ed25519 { algorithm, .. } => *algorithm,
        }
    }

    /// Get the cryptographic scheme.
    pub fn scheme(&self) -> crate::crypto::CryptoScheme {
        match self {
            Cypher::Sr25519 { .. } => crate::crypto::CryptoScheme::Sr25519,
            Cypher::Ed25519 { .. } => crate::crypto::CryptoScheme::Ed25519,
        }
    }

    /// Encrypt data for a specific receiver.
    ///
    /// # Arguments
    ///
    /// * `plaintext` - The data to encrypt
    /// * `receiver_public` - The recipient's public key (exactly 32 bytes, enforced at compile time)
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
    /// Returns error if encryption fails
    pub fn encrypt(&self, plaintext: &[u8], receiver_public: &[u8; 32]) -> Result<Vec<u8>> {
        match self {
            Cypher::Sr25519 { pair, algorithm } => {
                let receiver = sp_core::sr25519::Public::from_raw(*receiver_public);
                super::encryption::encrypt(plaintext, pair, &receiver, *algorithm)
            }
            Cypher::Ed25519 { pair, algorithm } => {
                let receiver = sp_core::ed25519::Public::from_raw(*receiver_public);
                super::encryption::encrypt(plaintext, pair, &receiver, *algorithm)
            }
        }
    }

    /// Decrypt data.
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - JSON-formatted encrypted data
    /// * `expected_sender` - Optional sender public key for verification (exactly 32 bytes, enforced at compile time)
    ///
    /// # Returns
    ///
    /// Returns decrypted plaintext bytes
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Decryption fails
    /// - Sender verification fails (if expected_sender provided)
    pub fn decrypt(&self, ciphertext: &[u8], expected_sender: Option<&[u8; 32]>) -> Result<Vec<u8>> {
        match self {
            Cypher::Sr25519 { pair, .. } => {
                let expected_public = expected_sender.map(|bytes| sp_core::sr25519::Public::from_raw(*bytes));
                super::encryption::decrypt(ciphertext, pair, expected_public.as_ref())
            }
            Cypher::Ed25519 { pair, .. } => {
                let expected_public = expected_sender.map(|bytes| sp_core::ed25519::Public::from_raw(*bytes));
                super::encryption::decrypt(ciphertext, pair, expected_public.as_ref())
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
        ).unwrap();
        
        assert_eq!(cypher.algorithm(), crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305);
        assert_eq!(cypher.scheme(), crate::crypto::CryptoScheme::Sr25519);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_sr25519() {
        let cypher = Cypher::new(
            "//Alice".to_string(),
            crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305,
            crate::crypto::CryptoScheme::Sr25519,
        ).unwrap();

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
        ).unwrap();

        // Get Alice's public key for self-encryption
        let pair = sp_core::ed25519::Pair::from_string("//Alice", None).unwrap();
        let public_key = pair.public().0;

        let plaintext = b"Hello, World!";
        let encrypted = cypher.encrypt(plaintext, &public_key).unwrap();
        let decrypted = cypher.decrypt(&encrypted, None).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }
}
