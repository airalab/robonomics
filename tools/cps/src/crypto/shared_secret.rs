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
//! Shared secret derivation for ECDH key agreement.
//!
//! This module provides a trait-based approach for deriving shared secrets
//! from different keypair types, supporting both SR25519 and ED25519.

use anyhow::{anyhow, Result};
use sp_core::crypto::Pair;

/// Shared secret derived from ECDH key agreement.
///
/// This struct holds a 32-byte shared secret that can be used to derive
/// encryption keys using HKDF.
#[derive(Clone)]
pub struct SharedSecret([u8; 32]);

impl SharedSecret {
    /// Create a new shared secret from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get a reference to the raw shared secret bytes.
    ///
    /// # Security Warning
    ///
    /// The shared secret should be treated as sensitive cryptographic material.
    /// Prefer using `derive_encryption_key()` instead of accessing raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Derive an encryption key from the shared secret using HKDF-SHA256.
    ///
    /// Uses HKDF (HMAC-based Key Derivation Function) with SHA256 to derive
    /// a 32-byte encryption key suitable for the specified algorithm.
    ///
    /// # Arguments
    ///
    /// * `info` - Application-specific context and purpose for the derived key
    ///
    /// # Returns
    ///
    /// Returns 32-byte encryption key
    ///
    /// # Errors
    ///
    /// Returns error if HKDF expansion fails
    pub fn derive_encryption_key(&self, info: &[u8]) -> Result<[u8; 32]> {
        use hkdf::Hkdf;
        use sha2::Sha256;

        let hkdf = Hkdf::<Sha256>::new(None, &self.0);
        let mut okm = [0u8; 32];
        hkdf.expand(info, &mut okm)
            .map_err(|e| anyhow!("HKDF expansion failed: {e}"))?;
        Ok(okm)
    }
}

impl AsRef<[u8]> for SharedSecret {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for SharedSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SharedSecret([REDACTED])")
    }
}

/// Trait for deriving shared secrets from keypair types.
///
/// This trait provides a common interface for ECDH key agreement
/// across different cryptographic curves and key types.
pub trait DeriveSharedSecret: Pair {
    /// Derive a shared secret using ECDH.
    ///
    /// # Arguments
    ///
    /// * `destination` - The destination's public key
    ///
    /// # Returns
    ///
    /// Returns a `SharedSecret` containing the derived 32-byte secret
    fn derive_secret(&self, destination: &Self::Public) -> Result<SharedSecret>;
}

/// SR25519 keypair implementation of shared secret derivation.
///
/// Uses Ristretto255 curve for ECDH key agreement.
impl DeriveSharedSecret for sp_core::sr25519::Pair {
    fn derive_secret(&self, destination: &Self::Public) -> Result<SharedSecret> {
        use curve25519_dalek::ristretto::CompressedRistretto;
        use curve25519_dalek::scalar::Scalar;
        use sha2::{Digest, Sha512};

        // Get the secret key seed from the sender keypair
        let secret_bytes = self.to_raw_vec();
        let mut scalar_bytes = [0u8; 32];
        // SR25519 secret key is 64 bytes, we use the first 32 bytes as the scalar
        scalar_bytes.copy_from_slice(&secret_bytes[..32]);

        // Create scalar for Ristretto255
        let scalar = Scalar::from_bytes_mod_order(scalar_bytes);

        // Get destination public key as Ristretto point
        let public_bytes: [u8; 32] = destination.0;
        let public_compressed = CompressedRistretto(public_bytes);
        let public_point = public_compressed
            .decompress()
            .ok_or_else(|| anyhow!("Failed to decompress Ristretto255 public key"))?;

        // Perform scalar multiplication on Ristretto255
        let shared_point = scalar * public_point;

        // Compress and hash the result for uniform distribution
        let shared_compressed = shared_point.compress();

        let mut hasher = Sha512::new();
        hasher.update(b"robonomics-cps-ecdh");
        hasher.update(shared_compressed.as_bytes());
        let hash_output = hasher.finalize();

        let mut result = [0u8; 32];
        result.copy_from_slice(&hash_output[..32]);

        Ok(SharedSecret::from_bytes(result))
    }
}

/// ED25519 keypair implementation of shared secret derivation.
///
/// Uses X25519 ECDH via ED25519 → Curve25519 conversion.
impl DeriveSharedSecret for sp_core::ed25519::Pair {
    fn derive_secret(&self, destination: &Self::Public) -> Result<SharedSecret> {
        // Convert ED25519 keys to X25519 for ECDH
        // ED25519 and Curve25519 are birationally equivalent curves

        // Get the secret key seed (first 32 bytes)
        let secret_bytes = self.to_raw_vec();
        let mut secret_seed = [0u8; 32];
        secret_seed.copy_from_slice(&secret_bytes[..32]);

        // For ED25519, we need to hash and clamp the seed to get the scalar
        use sha2::{Digest, Sha512};
        let mut hasher = Sha512::new();
        hasher.update(secret_seed);
        let hash = hasher.finalize();

        let mut scalar_bytes = [0u8; 32];
        scalar_bytes.copy_from_slice(&hash[..32]);

        // Clamp the scalar for X25519
        scalar_bytes[0] &= 248;
        scalar_bytes[31] &= 127;
        scalar_bytes[31] |= 64;

        let my_x25519_secret = x25519_dalek::StaticSecret::from(scalar_bytes);

        // Convert ED25519 public key to X25519 public key
        // The Montgomery u-coordinate can be derived from the Edwards y-coordinate
        let ed_public_bytes: [u8; 32] = destination.0;

        // Convert Edwards point to Montgomery point
        // This is a standard conversion: u = (1 + y) / (1 - y)
        use curve25519_dalek::edwards::CompressedEdwardsY;
        let compressed_edwards = CompressedEdwardsY(ed_public_bytes);
        let edwards_point = compressed_edwards
            .decompress()
            .ok_or_else(|| anyhow!("Failed to decompress ED25519 public key"))?;

        let montgomery_point = edwards_point.to_montgomery();
        let their_x25519_public = x25519_dalek::PublicKey::from(montgomery_point.to_bytes());

        // Perform X25519 ECDH
        let shared_secret = my_x25519_secret.diffie_hellman(&their_x25519_public);

        Ok(SharedSecret::from_bytes(*shared_secret.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sr25519_shared_secret() {
        // Create two keypairs
        let (alice, _) = sp_core::sr25519::Pair::generate();
        let (bob, _) = sp_core::sr25519::Pair::generate();

        // Derive shared secrets from both sides
        let shared_alice_bob = alice.derive_secret(&bob.public()).unwrap();
        let shared_bob_alice = bob.derive_secret(&alice.public()).unwrap();

        // Shared secrets should be identical
        assert_eq!(shared_alice_bob.as_bytes(), shared_bob_alice.as_bytes());

        // Shared secret should not be all zeros
        assert_ne!(shared_alice_bob.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn test_ed25519_shared_secret() {
        // Create two keypairs
        let (alice, _) = sp_core::ed25519::Pair::generate();
        let (bob, _) = sp_core::ed25519::Pair::generate();

        // Derive shared secrets from both sides
        let shared_alice_bob = alice.derive_secret(&bob.public()).unwrap();
        let shared_bob_alice = bob.derive_secret(&alice.public()).unwrap();

        // Shared secrets should be identical
        assert_eq!(shared_alice_bob.as_bytes(), shared_bob_alice.as_bytes());

        // Shared secret should not be all zeros
        assert_ne!(shared_alice_bob.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn test_shared_secret_derive_key() {
        let (alice, _) = sp_core::sr25519::Pair::generate();
        let (bob, _) = sp_core::sr25519::Pair::generate();

        let shared = alice.derive_secret(&bob.public()).unwrap();

        // Derive encryption key
        let key = shared
            .derive_encryption_key(crate::crypto::EncryptionAlgorithm::XChaCha20Poly1305.info_string())
            .unwrap();

        // Key should be 32 bytes
        assert_eq!(key.len(), 32);

        // Key should be different from shared secret
        assert_ne!(&key, shared.as_bytes());
    }

    #[test]
    fn test_different_keypair_types_different_secrets() {
        // Use same seed for both types
        let seed = [42u8; 32];

        let sr_alice = sp_core::sr25519::Pair::from_seed(&seed);
        let sr_bob = sp_core::sr25519::Pair::from_seed(&[43u8; 32]);

        let ed_alice = sp_core::ed25519::Pair::from_seed(&seed);
        let ed_bob = sp_core::ed25519::Pair::from_seed(&[43u8; 32]);

        let shared_sr = sr_alice.derive_secret(&sr_bob.public()).unwrap();
        let shared_ed = ed_alice.derive_secret(&ed_bob.public()).unwrap();

        // Different curve operations should produce different shared secrets
        assert_ne!(shared_sr.as_bytes(), shared_ed.as_bytes());
    }
}
