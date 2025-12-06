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
//! Encryption algorithm selection and configuration.
//!
//! This module provides types for selecting and configuring different AEAD
//! encryption algorithms supported by the CPS pallet.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Supported AEAD encryption algorithms.
///
/// Each algorithm provides authenticated encryption with associated data (AEAD),
/// ensuring both confidentiality and integrity of encrypted data.
///
/// # Algorithms
///
/// - **XChaCha20Poly1305**: Extended nonce ChaCha20-Poly1305 (24-byte nonce)
///   - Best for: General purpose, portable, large nonce space
///   - Performance: ~680 MB/s (software)
///   - Nonce: 192 bits (collision-resistant)
///
/// - **AesGcm256**: AES-256 in Galois/Counter Mode (12-byte nonce)
///   - Best for: Hardware acceleration (AES-NI), high throughput
///   - Performance: ~2-3 GB/s (with AES-NI)
///   - Nonce: 96 bits (requires careful management)
///
/// - **ChaCha20Poly1305**: Standard ChaCha20-Poly1305 (12-byte nonce)
///   - Best for: Portable performance without hardware acceleration
///   - Performance: ~600 MB/s (software)
///   - Nonce: 96 bits (requires careful management)
///
/// # Examples
///
/// ```
/// use libcps::crypto::EncryptionAlgorithm;
/// use std::str::FromStr;
///
/// let algo = EncryptionAlgorithm::XChaCha20Poly1305;
/// assert_eq!(algo.info_string(), b"robonomics-cps-xchacha20poly1305");
///
/// let from_str = EncryptionAlgorithm::from_str("aesgcm256").unwrap();
/// assert_eq!(from_str, EncryptionAlgorithm::AesGcm256);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    /// assert_eq!(algo.info_string(), b"robonomics-cps-xchacha20poly1305");
    /// ```
    pub fn info_string(&self) -> &'static [u8] {
        match self {
            Self::XChaCha20Poly1305 => b"robonomics-cps-xchacha20poly1305",
            Self::AesGcm256 => b"robonomics-cps-aesgcm256",
            Self::ChaCha20Poly1305 => b"robonomics-cps-chacha20poly1305",
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
        write!(f, "{}", self.name())
    }
}

impl FromStr for EncryptionAlgorithm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "xchacha20" | "xchacha20poly1305" | "xchacha20-poly1305" => {
                Ok(Self::XChaCha20Poly1305)
            }
            "aesgcm256" | "aes-256-gcm" | "aes256gcm" | "aesgcm" => Ok(Self::AesGcm256),
            "chacha20" | "chacha20poly1305" | "chacha20-poly1305" => Ok(Self::ChaCha20Poly1305),
            _ => Err(format!(
                "Unknown encryption algorithm: '{}'. Supported: xchacha20, aesgcm256, chacha20",
                s
            )),
        }
    }
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
            b"robonomics-cps-xchacha20poly1305"
        );
        assert_eq!(
            EncryptionAlgorithm::AesGcm256.info_string(),
            b"robonomics-cps-aesgcm256"
        );
        assert_eq!(
            EncryptionAlgorithm::ChaCha20Poly1305.info_string(),
            b"robonomics-cps-chacha20poly1305"
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
}
