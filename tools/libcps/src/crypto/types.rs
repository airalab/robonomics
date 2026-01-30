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
//! Cryptographic types for encryption.
//!
//! This module provides types for cryptographic schemes, encryption algorithms,
//! and encrypted message formats used throughout the library.

use parity_scale_codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Supported cryptographic schemes for encryption.
///
/// This enum distinguishes between different cryptographic key types
/// used for ECDH key agreement and encryption, following Polkadot's
/// naming convention of "scheme" rather than "keypair type".
///
/// # Schemes
///
/// - **SR25519**: Schnorrkel-based keys using Ristretto255 (Substrate native)
///   - Used in: Substrate/Polkadot ecosystem
///   - Key agreement: Ristretto255 scalar multiplication
///   - Best for: Substrate blockchain operations
///
/// - **ED25519**: Edwards curve keys with X25519 ECDH conversion
///   - Used in: IoT devices, Home Assistant, standard cryptography
///   - Key agreement: ED25519 → Curve25519 → X25519
///   - Best for: Compatibility with standard ED25519 implementations
///
/// # Examples
///
/// ```
/// use libcps::crypto::CryptoScheme;
/// use std::str::FromStr;
///
/// let scheme = CryptoScheme::Sr25519;
/// assert_eq!(scheme.to_string(), "sr25519");
///
/// let from_str = CryptoScheme::from_str("ed25519").unwrap();
/// assert_eq!(from_str, CryptoScheme::Ed25519);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CryptoScheme {
    /// Schnorrkel SR25519 keys (Substrate native)
    Sr25519,
    /// ED25519 keys (common in IoT, Home Assistant)
    Ed25519,
}

impl CryptoScheme {
    /// Get a human-readable name for this cryptographic scheme.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::crypto::CryptoScheme;
    ///
    /// assert_eq!(CryptoScheme::Sr25519.name(), "SR25519");
    /// assert_eq!(CryptoScheme::Ed25519.name(), "ED25519");
    /// ```
    pub fn name(&self) -> &'static str {
        match self {
            Self::Sr25519 => "SR25519",
            Self::Ed25519 => "ED25519",
        }
    }

    /// Get the info string suffix for HKDF key derivation.
    ///
    /// This is used for domain separation in HKDF, ensuring keys derived
    /// for different schemes are independent.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::crypto::CryptoScheme;
    ///
    /// assert_eq!(CryptoScheme::Sr25519.info_suffix(), "sr25519");
    /// assert_eq!(CryptoScheme::Ed25519.info_suffix(), "ed25519");
    /// ```
    pub fn info_suffix(&self) -> &'static str {
        match self {
            Self::Sr25519 => "sr25519",
            Self::Ed25519 => "ed25519",
        }
    }
}

impl Default for CryptoScheme {
    fn default() -> Self {
        Self::Sr25519
    }
}

impl fmt::Display for CryptoScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoScheme::Sr25519 => write!(f, "sr25519"),
            CryptoScheme::Ed25519 => write!(f, "ed25519"),
        }
    }
}

impl FromStr for CryptoScheme {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sr25519" | "sr" => Ok(CryptoScheme::Sr25519),
            "ed25519" | "ed" => Ok(CryptoScheme::Ed25519),
            _ => Err(anyhow::anyhow!(
                "Invalid cryptographic scheme: '{s}'. Supported: sr25519, ed25519"
            )),
        }
    }
}

/// Supported encryption algorithms.
///
/// All algorithms use AEAD (Authenticated Encryption with Associated Data)
/// to provide both confidentiality and authenticity.
///
/// # Examples
///
/// ```
/// use libcps::crypto::EncryptionAlgorithm;
///
/// let algo = EncryptionAlgorithm::XChaCha20Poly1305;
/// assert_eq!(algo.name(), "XChaCha20-Poly1305");
/// assert_eq!(algo.nonce_size(), 24);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// XChaCha20-Poly1305 AEAD (24-byte nonce)
    XChaCha20Poly1305,
    /// AES-256-GCM AEAD (12-byte nonce)
    AesGcm256,
    /// ChaCha20-Poly1305 AEAD (12-byte nonce)
    ChaCha20Poly1305,
}

impl EncryptionAlgorithm {
    /// Get the HKDF info string for this algorithm.
    ///
    /// The info string is used for domain separation in HKDF key derivation,
    /// ensuring keys derived for different algorithms are independent.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::crypto::EncryptionAlgorithm;
    ///
    /// let algo = EncryptionAlgorithm::XChaCha20Poly1305;
    /// assert_eq!(algo.info_string(), "xchacha20poly1305");
    /// ```
    pub fn info_string(&self) -> &'static str {
        match self {
            Self::XChaCha20Poly1305 => "xchacha20poly1305",
            Self::AesGcm256 => "aesgcm256",
            Self::ChaCha20Poly1305 => "chacha20poly1305",
        }
    }

    /// Get the nonce size in bytes for this algorithm.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::crypto::EncryptionAlgorithm;
    ///
    /// assert_eq!(EncryptionAlgorithm::XChaCha20Poly1305.nonce_size(), 24);
    /// assert_eq!(EncryptionAlgorithm::AesGcm256.nonce_size(), 12);
    /// assert_eq!(EncryptionAlgorithm::ChaCha20Poly1305.nonce_size(), 12);
    /// ```
    pub fn nonce_size(&self) -> usize {
        match self {
            Self::XChaCha20Poly1305 => 24,
            Self::AesGcm256 => 12,
            Self::ChaCha20Poly1305 => 12,
        }
    }

    /// Get the key size in bytes for this algorithm.
    ///
    /// All supported algorithms use 256-bit (32-byte) keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::crypto::EncryptionAlgorithm;
    ///
    /// assert_eq!(EncryptionAlgorithm::XChaCha20Poly1305.key_size(), 32);
    /// ```
    pub fn key_size(&self) -> usize {
        32 // All algorithms use 256-bit keys
    }

    /// Get a human-readable name for this algorithm.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::crypto::EncryptionAlgorithm;
    ///
    /// assert_eq!(EncryptionAlgorithm::XChaCha20Poly1305.name(), "XChaCha20-Poly1305");
    /// ```
    pub fn name(&self) -> &'static str {
        match self {
            Self::XChaCha20Poly1305 => "XChaCha20-Poly1305",
            Self::AesGcm256 => "AES-256-GCM",
            Self::ChaCha20Poly1305 => "ChaCha20-Poly1305",
        }
    }
}

impl Default for EncryptionAlgorithm {
    fn default() -> Self {
        Self::XChaCha20Poly1305
    }
}

impl fmt::Display for EncryptionAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.info_string())
    }
}

impl FromStr for EncryptionAlgorithm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "xchacha20" | "xchacha20poly1305" | "xchacha20-poly1305" => Ok(Self::XChaCha20Poly1305),
            "aesgcm256" | "aes-256-gcm" | "aes256gcm" | "aesgcm" => Ok(Self::AesGcm256),
            "chacha20" | "chacha20poly1305" | "chacha20-poly1305" => Ok(Self::ChaCha20Poly1305),
            _ => Err(format!(
                "Unknown encryption algorithm: '{s}'. Supported: xchacha20, aesgcm256, chacha20"
            )),
        }
    }
}

/// Encrypted message format stored on-chain.
///
/// This enum is versioned to support future format changes while maintaining
/// backward compatibility. Uses SCALE codec for efficient binary serialization.
#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub enum EncryptedMessage {
    /// Version 1 of the encrypted message format.
    ///
    /// Uses AEAD encryption with algorithm identifier, sender's public key,
    /// nonce, and ciphertext as binary data.
    V1 {
        /// Encryption algorithm
        algorithm: EncryptionAlgorithm,
        /// Sender's public key (32 bytes)
        #[serde(with = "easy_hex::serde")]
        from: [u8; 32],
        /// Nonce for the encryption (size varies by algorithm: 24 bytes for XChaCha20, 12 for AES-GCM/ChaCha20)
        #[serde(with = "easy_hex::serde")]
        nonce: Vec<u8>,
        /// Encrypted ciphertext with authentication tag
        #[serde(with = "easy_hex::serde")]
        ciphertext: Vec<u8>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_algorithm_from_str() {
        assert_eq!(
            EncryptionAlgorithm::from_str("xchacha20").unwrap(),
            EncryptionAlgorithm::XChaCha20Poly1305
        );
        assert_eq!(
            EncryptionAlgorithm::from_str("aesgcm256").unwrap(),
            EncryptionAlgorithm::AesGcm256
        );
        assert_eq!(
            EncryptionAlgorithm::from_str("chacha20").unwrap(),
            EncryptionAlgorithm::ChaCha20Poly1305
        );
    }

    #[test]
    fn test_algorithm_info_strings() {
        assert_eq!(
            EncryptionAlgorithm::XChaCha20Poly1305.info_string(),
            "xchacha20poly1305"
        );
        assert_eq!(EncryptionAlgorithm::AesGcm256.info_string(), "aesgcm256");
        assert_eq!(
            EncryptionAlgorithm::ChaCha20Poly1305.info_string(),
            "chacha20poly1305"
        );
    }

    #[test]
    fn test_nonce_sizes() {
        assert_eq!(EncryptionAlgorithm::XChaCha20Poly1305.nonce_size(), 24);
        assert_eq!(EncryptionAlgorithm::AesGcm256.nonce_size(), 12);
        assert_eq!(EncryptionAlgorithm::ChaCha20Poly1305.nonce_size(), 12);
    }

    #[test]
    fn test_default() {
        assert_eq!(
            EncryptionAlgorithm::default(),
            EncryptionAlgorithm::XChaCha20Poly1305
        );
    }

    #[test]
    fn test_crypto_scheme_name() {
        assert_eq!(CryptoScheme::Sr25519.name(), "SR25519");
        assert_eq!(CryptoScheme::Ed25519.name(), "ED25519");
    }

    #[test]
    fn test_crypto_scheme_info_suffix() {
        assert_eq!(CryptoScheme::Sr25519.info_suffix(), "-sr25519");
        assert_eq!(CryptoScheme::Ed25519.info_suffix(), "-ed25519");
    }

    #[test]
    fn test_crypto_scheme_display() {
        assert_eq!(CryptoScheme::Sr25519.to_string(), "sr25519");
        assert_eq!(CryptoScheme::Ed25519.to_string(), "ed25519");
    }

    #[test]
    fn test_crypto_scheme_from_str() {
        assert_eq!(
            CryptoScheme::from_str("sr25519").unwrap(),
            CryptoScheme::Sr25519
        );
        assert_eq!(
            CryptoScheme::from_str("ed25519").unwrap(),
            CryptoScheme::Ed25519
        );
        assert!(CryptoScheme::from_str("invalid").is_err());
    }
}
