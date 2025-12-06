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
//! Keypair type selection for encryption.
//!
//! This module provides types for selecting between SR25519 and ED25519
//! keypair types for encryption operations.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Supported keypair types for encryption.
///
/// This enum distinguishes between different cryptographic key types
/// used for ECDH key agreement and encryption.
///
/// # Key Types
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
/// use libcps::crypto::KeypairType;
/// use std::str::FromStr;
///
/// let keypair_type = KeypairType::Sr25519;
/// assert_eq!(keypair_type.to_string(), "sr25519");
///
/// let from_str = KeypairType::from_str("ed25519").unwrap();
/// assert_eq!(from_str, KeypairType::Ed25519);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeypairType {
    /// Schnorrkel SR25519 keys (Substrate native)
    Sr25519,
    /// ED25519 keys (common in IoT, Home Assistant)
    Ed25519,
}

impl KeypairType {
    /// Get a human-readable name for this keypair type.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::crypto::KeypairType;
    ///
    /// assert_eq!(KeypairType::Sr25519.name(), "SR25519");
    /// assert_eq!(KeypairType::Ed25519.name(), "ED25519");
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
    /// for different keypair types are independent.
    ///
    /// # Examples
    ///
    /// ```
    /// use libcps::crypto::KeypairType;
    ///
    /// assert_eq!(KeypairType::Sr25519.info_suffix(), "sr25519");
    /// assert_eq!(KeypairType::Ed25519.info_suffix(), "ed25519");
    /// ```
    pub fn info_suffix(&self) -> &'static str {
        match self {
            Self::Sr25519 => "sr25519",
            Self::Ed25519 => "ed25519",
        }
    }
}

impl Default for KeypairType {
    fn default() -> Self {
        Self::Sr25519
    }
}

impl fmt::Display for KeypairType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeypairType::Sr25519 => write!(f, "sr25519"),
            KeypairType::Ed25519 => write!(f, "ed25519"),
        }
    }
}

impl FromStr for KeypairType {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sr25519" | "sr" => Ok(KeypairType::Sr25519),
            "ed25519" | "ed" => Ok(KeypairType::Ed25519),
            _ => Err(anyhow::anyhow!("Invalid keypair type: '{}'. Supported: sr25519, ed25519", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_type_from_str() {
        assert_eq!(
            KeypairType::from_str("sr25519").unwrap(),
            KeypairType::Sr25519
        );
        assert_eq!(
            KeypairType::from_str("sr").unwrap(),
            KeypairType::Sr25519
        );
        assert_eq!(
            KeypairType::from_str("ed25519").unwrap(),
            KeypairType::Ed25519
        );
        assert_eq!(
            KeypairType::from_str("ed").unwrap(),
            KeypairType::Ed25519
        );
        assert_eq!(
            KeypairType::from_str("SR25519").unwrap(),
            KeypairType::Sr25519
        );
        assert_eq!(
            KeypairType::from_str("ED25519").unwrap(),
            KeypairType::Ed25519
        );
    }

    #[test]
    fn test_keypair_type_from_str_invalid() {
        assert!(KeypairType::from_str("invalid").is_err());
        assert!(KeypairType::from_str("secp256k1").is_err());
    }

    #[test]
    fn test_keypair_type_display() {
        assert_eq!(KeypairType::Sr25519.to_string(), "sr25519");
        assert_eq!(KeypairType::Ed25519.to_string(), "ed25519");
    }

    #[test]
    fn test_keypair_type_name() {
        assert_eq!(KeypairType::Sr25519.name(), "SR25519");
        assert_eq!(KeypairType::Ed25519.name(), "ED25519");
    }

    #[test]
    fn test_keypair_type_info_suffix() {
        assert_eq!(KeypairType::Sr25519.info_suffix(), "sr25519");
        assert_eq!(KeypairType::Ed25519.info_suffix(), "ed25519");
    }

    #[test]
    fn test_default() {
        assert_eq!(KeypairType::default(), KeypairType::Sr25519);
    }

    #[test]
    fn test_serialization() {
        let sr25519 = KeypairType::Sr25519;
        let json = serde_json::to_string(&sr25519).unwrap();
        assert_eq!(json, "\"sr25519\"");

        let ed25519 = KeypairType::Ed25519;
        let json = serde_json::to_string(&ed25519).unwrap();
        assert_eq!(json, "\"ed25519\"");
    }

    #[test]
    fn test_deserialization() {
        let sr25519: KeypairType = serde_json::from_str("\"sr25519\"").unwrap();
        assert_eq!(sr25519, KeypairType::Sr25519);

        let ed25519: KeypairType = serde_json::from_str("\"ed25519\"").unwrap();
        assert_eq!(ed25519, KeypairType::Ed25519);
    }
}
