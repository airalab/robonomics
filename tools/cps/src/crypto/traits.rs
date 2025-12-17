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
//! Encryption and decryption traits for the CPS library.
//!
//! This module provides generic traits for encryption and decryption operations,
//! allowing different implementations for various key types and algorithms.

use anyhow::Result;

/// Trait for types that can encrypt data.
///
/// This trait abstracts encryption operations, allowing different
/// key types and algorithms to provide their own implementations.
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::{Encrypt, EncryptionAlgorithm, CryptoScheme};
/// use anyhow::Result;
///
/// fn encrypt_data<E: Encrypt>(
///     encryptor: &E,
///     plaintext: &[u8],
///     algorithm: EncryptionAlgorithm,
///     scheme: CryptoScheme,
/// ) -> Result<Vec<u8>> {
///     encryptor.encrypt(plaintext, algorithm, scheme)
/// }
/// ```
pub trait Encrypt {
    /// Encrypt plaintext using the specified algorithm and cryptographic scheme.
    ///
    /// # Arguments
    ///
    /// * `plaintext` - The data to encrypt
    /// * `algorithm` - The AEAD cipher algorithm to use
    /// * `scheme` - The cryptographic scheme (sr25519, ed25519)
    ///
    /// # Returns
    ///
    /// Returns encrypted bytes in JSON format containing:
    /// - version: Format version
    /// - algorithm: Algorithm identifier
    /// - from: Sender's public key (base58)
    /// - nonce: Base64-encoded nonce
    /// - ciphertext: Base64-encoded encrypted data
    fn encrypt(
        &self,
        plaintext: &[u8],
        algorithm: crate::crypto::EncryptionAlgorithm,
        scheme: crate::crypto::CryptoScheme,
    ) -> Result<Vec<u8>>;
}

/// Trait for types that can decrypt data.
///
/// This trait abstracts decryption operations, allowing different
/// key types to provide their own implementations.
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::{Decrypt, CryptoScheme};
/// use anyhow::Result;
///
/// fn decrypt_data<D: Decrypt>(
///     decryptor: &D,
///     ciphertext: &[u8],
///     scheme: CryptoScheme,
/// ) -> Result<Vec<u8>> {
///     decryptor.decrypt(ciphertext, scheme, None)
/// }
/// ```
pub trait Decrypt {
    /// Decrypt ciphertext using the specified cryptographic scheme.
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - JSON-formatted encrypted data
    /// * `scheme` - The cryptographic scheme (sr25519, ed25519)
    /// * `expected_sender` - Optional sender public key for verification
    ///
    /// # Returns
    ///
    /// Returns decrypted plaintext bytes
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - JSON parsing fails
    /// - Algorithm is unsupported
    /// - ECDH key agreement fails
    /// - Decryption fails
    /// - Sender verification fails (if expected_sender provided)
    fn decrypt(
        &self,
        ciphertext: &[u8],
        scheme: crate::crypto::CryptoScheme,
        expected_sender: Option<&[u8]>,
    ) -> Result<Vec<u8>>;
}
