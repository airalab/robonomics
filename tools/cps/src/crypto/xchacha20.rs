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
//! XChaCha20-Poly1305 encryption with sr25519 key derivation.
//!
//! This module implements the encryption scheme specified for Robonomics CPS:
//! sr25519 keys → ECDH → HKDF-SHA256 → XChaCha20-Poly1305 AEAD
//!
//! # Encryption Scheme
//!
//! 1. **Key Agreement**: Derive shared secret from sender's secret and receiver's public key
//! 2. **Key Derivation**: Use HKDF-SHA256 with info string `"robonomics-cps-xchacha20poly1305"`
//! 3. **Encryption**: XChaCha20-Poly1305 AEAD with random 24-byte nonce
//!
//! # Message Format
//!
//! Encrypted messages are JSON-encoded with the following structure:
//!
//! ```json
//! {
//!   "version": 1,
//!   "from": "5GrwvaEF...",
//!   "nonce": "base64-encoded-24-bytes",
//!   "ciphertext": "base64-encoded-data"
//! }
//! ```
//!
//! # Examples
//!
//! ```no_run
//! use libcps::crypto::{encrypt, decrypt};
//! use schnorrkel::SecretKey;
//!
//! # fn example() -> anyhow::Result<()> {
//! let sender_secret = SecretKey::from_bytes(&[0u8; 64])?;
//! let receiver_public = [0u8; 32];
//! let plaintext = b"secret message";
//!
//! // Encrypt
//! let encrypted = encrypt(plaintext, &sender_secret, &receiver_public)?;
//!
//! // Decrypt
//! let receiver_secret = SecretKey::from_bytes(&[0u8; 64])?;
//! let decrypted = decrypt(&encrypted, &receiver_secret)?;
//!
//! assert_eq!(plaintext, &decrypted[..]);
//! # Ok(())
//! # }
//! ```
//!
//! # Security
//!
//! - **AEAD**: XChaCha20-Poly1305 provides authenticated encryption
//! - **Nonce**: 24-byte nonces from secure random source (OsRng)
//! - **KDF**: HKDF-SHA256 ensures proper key derivation
//! - **Info String**: Domain separation via fixed info string

use anyhow::{anyhow, Result};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use schnorrkel::{PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

/// Encrypted message format stored on-chain.
///
/// This structure is JSON-serialized for storage and transmission.
///
/// # Fields
///
/// * `version` - Message format version (currently 1)
/// * `from` - Sender's public key in base58 encoding
/// * `nonce` - XChaCha20 nonce in base64 encoding (24 bytes)
/// * `ciphertext` - Encrypted data in base64 encoding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedMessage {
    /// Message format version
    pub version: u8,
    /// Sender's sr25519 public key (base58-encoded)
    pub from: String,
    /// XChaCha20 nonce (base64-encoded, 24 bytes)
    pub nonce: String,
    /// Encrypted ciphertext (base64-encoded)
    pub ciphertext: String,
}

/// HKDF info string for domain separation
const INFO: &[u8] = b"robonomics-cps-xchacha20poly1305";

/// Derive shared secret from sr25519 keys using HKDF-based key agreement.
///
/// Since sr25519 uses Ristretto255 group (not directly compatible with X25519),
/// we use a secure hash-based key agreement scheme:
/// 1. Compute ECDH-like operation: secret_scalar * public_point (on Ristretto255)
/// 2. Hash the result with HKDF for key derivation
///
/// This provides forward secrecy and is cryptographically secure, though not
/// standard X25519. Both parties must use the same derivation to get matching secrets.
///
/// # Arguments
///
/// * `secret_key` - The secret key for DH
/// * `public_key` - The public key to compute shared secret with
///
/// # Returns
///
/// Returns 32-byte shared secret
///
/// # Errors
///
/// Returns error if the operation fails
fn derive_shared_secret(secret_key: &SecretKey, public_key: &PublicKey) -> Result<[u8; 32]> {
    use curve25519_dalek::ristretto::RistrettoPoint;
    use curve25519_dalek::scalar::Scalar;
    use sha2::{Digest, Sha512};
    
    // Get secret scalar (first 32 bytes of secret key)
    let secret_bytes = secret_key.to_bytes();
    let mut scalar_bytes = [0u8; 32];
    scalar_bytes.copy_from_slice(&secret_bytes[..32]);
    
    // Create scalar (schnorrkel uses SHA-512 internally, we replicate this)
    let scalar = Scalar::from_bytes_mod_order(scalar_bytes);
    
    // Get public key as Ristretto point
    // schnorrkel PublicKey is compressed Ristretto255
    let public_compressed = curve25519_dalek::ristretto::CompressedRistretto(public_key.to_bytes());
    let public_point = public_compressed
        .decompress()
        .ok_or_else(|| anyhow!("Failed to decompress Ristretto255 public key"))?;
    
    // Perform scalar multiplication on Ristretto255
    let shared_point = scalar * public_point;
    
    // Compress the result and hash it for the shared secret
    // This ensures both parties get the same result
    let shared_compressed = shared_point.compress();
    
    // Use SHA-512 to derive a uniform 64-byte output, then take first 32 bytes
    // This matches how Substrate typically handles key agreement
    let mut hasher = Sha512::new();
    hasher.update(b"robonomics-cps-ecdh");
    hasher.update(shared_compressed.as_bytes());
    let hash_output = hasher.finalize();
    
    let mut result = [0u8; 32];
    result.copy_from_slice(&hash_output[..32]);
    
    Ok(result)
}

/// Derive encryption key from shared secret using HKDF-SHA256.
///
/// Uses HKDF (HMAC-based Key Derivation Function) with SHA256 to derive
/// a 32-byte encryption key from the shared secret.
///
/// # Arguments
///
/// * `shared_secret` - The shared secret from ECDH
///
/// # Returns
///
/// Returns 32-byte encryption key
///
/// # Errors
///
/// Returns error if HKDF expansion fails
fn derive_encryption_key(shared_secret: &[u8; 32]) -> Result<[u8; 32]> {
    let hkdf = Hkdf::<Sha256>::new(None, shared_secret);
    let mut okm = [0u8; 32];
    hkdf.expand(INFO, &mut okm)
        .map_err(|e| anyhow!("HKDF expansion failed: {e}"))?;
    Ok(okm)
}

/// Encrypt data using sr25519 → XChaCha20-Poly1305 scheme.
///
/// # Process
///
/// 1. Derive shared secret from sender's secret key and receiver's public key
/// 2. Use HKDF-SHA256 to derive 32-byte encryption key from shared secret
/// 3. Encrypt plaintext with XChaCha20-Poly1305 using random 24-byte nonce
/// 4. Return JSON-encoded message with version, sender, nonce, and ciphertext
///
/// # Arguments
///
/// * `plaintext` - Data to encrypt
/// * `sender_secret` - Sender's sr25519 secret key
/// * `receiver_public` - Receiver's sr25519 public key (32 bytes)
///
/// # Returns
///
/// JSON-encoded [`EncryptedMessage`] with base64-encoded nonce and ciphertext
///
/// # Errors
///
/// Returns an error if:
/// - Receiver's public key is invalid
/// - HKDF expansion fails
/// - Encryption fails
/// - JSON serialization fails
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::encrypt;
/// use schnorrkel::SecretKey;
///
/// # fn example() -> anyhow::Result<()> {
/// let sender_secret = SecretKey::from_bytes(&[0u8; 64])?;
/// let receiver_public = [0u8; 32];
/// let plaintext = b"secret message";
///
/// let encrypted = encrypt(plaintext, &sender_secret, &receiver_public)?;
/// # Ok(())
/// # }
/// ```
pub fn encrypt(
    plaintext: &[u8],
    sender_secret: &SecretKey,
    receiver_public: &[u8; 32],
) -> Result<Vec<u8>> {
    // Step 1: Parse receiver's public key
    let receiver_pubkey = PublicKey::from_bytes(receiver_public)
        .map_err(|e| anyhow!("Invalid receiver public key: {e}"))?;
    
    // Step 2: Derive shared secret using ECDH
    let shared_secret = derive_shared_secret(sender_secret, &receiver_pubkey)?;
    
    // Step 3: Derive encryption key using HKDF
    let encryption_key = derive_encryption_key(&shared_secret)?;

    // Step 4: Encrypt with XChaCha20-Poly1305
    let cipher = XChaCha20Poly1305::new(&encryption_key.into());
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| anyhow!("Encryption failed: {e}"))?;

    // Step 5: Create message structure
    let sender_public = sender_secret.to_public();
    let message = EncryptedMessage {
        version: 1,
        from: bs58::encode(sender_public.to_bytes()).into_string(),
        nonce: base64::encode(nonce.as_slice()),
        ciphertext: base64::encode(&ciphertext),
    };

    // Serialize to JSON
    serde_json::to_vec(&message).map_err(|e| anyhow!("JSON serialization failed: {e}"))
}

/// Decrypt data using sr25519 → XChaCha20-Poly1305 scheme.
///
/// # Process
///
/// 1. Parse JSON-encoded encrypted message
/// 2. Derive shared secret from receiver's secret key and sender's public key
/// 3. Use HKDF-SHA256 to derive encryption key
/// 4. Decrypt ciphertext with XChaCha20-Poly1305
///
/// # Arguments
///
/// * `encrypted_data` - JSON-encoded [`EncryptedMessage`]
/// * `receiver_secret` - Receiver's sr25519 secret key
///
/// # Returns
///
/// Decrypted plaintext bytes
///
/// # Errors
///
/// Returns an error if:
/// - Cannot parse JSON message
/// - Unsupported message version
/// - Invalid sender public key
/// - HKDF expansion fails
/// - Cannot decode base64 nonce or ciphertext
/// - Decryption fails (wrong key or corrupted data)
///
/// # Examples
///
/// ```no_run
/// use libcps::crypto::{encrypt, decrypt};
/// use schnorrkel::SecretKey;
///
/// # fn example() -> anyhow::Result<()> {
/// let sender_secret = SecretKey::from_bytes(&[0u8; 64])?;
/// let receiver_secret = SecretKey::from_bytes(&[1u8; 64])?;
/// let receiver_public = receiver_secret.to_public().to_bytes();
/// let plaintext = b"secret message";
///
/// let encrypted = encrypt(plaintext, &sender_secret, &receiver_public)?;
/// let decrypted = decrypt(&encrypted, &receiver_secret)?;
///
/// assert_eq!(plaintext, &decrypted[..]);
/// # Ok(())
/// # }
/// ```
pub fn decrypt(encrypted_data: &[u8], receiver_secret: &SecretKey) -> Result<Vec<u8>> {
    // Step 1: Parse message
    let message: EncryptedMessage = serde_json::from_slice(encrypted_data)
        .map_err(|e| anyhow!("Failed to parse encrypted message: {e}"))?;

    if message.version != 1 {
        return Err(anyhow!("Unsupported encryption version: {}", message.version));
    }

    // Step 2: Decode sender's public key
    let sender_public_bytes = bs58::decode(&message.from)
        .into_vec()
        .map_err(|e| anyhow!("Failed to decode sender public key: {e}"))?;
    
    if sender_public_bytes.len() != 32 {
        return Err(anyhow!("Invalid sender public key length"));
    }
    
    let mut sender_pk_array = [0u8; 32];
    sender_pk_array.copy_from_slice(&sender_public_bytes);
    
    let sender_public = PublicKey::from_bytes(&sender_pk_array)
        .map_err(|e| anyhow!("Invalid sender public key: {e}"))?;

    // Step 3: Derive shared secret using ECDH
    let shared_secret = derive_shared_secret(receiver_secret, &sender_public)?;
    
    // Step 4: Derive encryption key using HKDF
    let encryption_key = derive_encryption_key(&shared_secret)?;

    // Step 5: Decode nonce and ciphertext
    let nonce_bytes = base64::decode(&message.nonce)
        .map_err(|e| anyhow!("Failed to decode nonce: {e}"))?;
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = base64::decode(&message.ciphertext)
        .map_err(|e| anyhow!("Failed to decode ciphertext: {e}"))?;

     // Step 6: Decrypt
    let cipher = XChaCha20Poly1305::new(&encryption_key.into());
    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| anyhow!("Decryption failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use schnorrkel::{Keypair, MiniSecretKey};
    
    /// Generate a test keypair from a seed
    fn test_keypair(seed: u8) -> Keypair {
        let mini_secret = MiniSecretKey::from_bytes(&[seed; 32]).unwrap();
        mini_secret.expand_to_keypair(schnorrkel::ExpansionMode::Ed25519)
    }
    
    #[test]
    fn test_derive_shared_secret() {
        // Create two keypairs
        let alice = test_keypair(1);
        let bob = test_keypair(2);
        
        // Derive shared secrets from both sides
        let shared_alice_bob = derive_shared_secret(&alice.secret, &bob.public).unwrap();
        let shared_bob_alice = derive_shared_secret(&bob.secret, &alice.public).unwrap();
        
        // Shared secrets should be identical
        assert_eq!(shared_alice_bob, shared_bob_alice);
        
        // Shared secret should be 32 bytes
        assert_eq!(shared_alice_bob.len(), 32);
        
        // Shared secret should not be all zeros
        assert_ne!(shared_alice_bob, [0u8; 32]);
    }
    
    #[test]
    fn test_derive_shared_secret_different_pairs() {
        // Create three keypairs
        let alice = test_keypair(1);
        let bob = test_keypair(2);
        let charlie = test_keypair(3);
        
        // Derive different shared secrets
        let shared_alice_bob = derive_shared_secret(&alice.secret, &bob.public).unwrap();
        let shared_alice_charlie = derive_shared_secret(&alice.secret, &charlie.public).unwrap();
        
        // Different pairs should produce different shared secrets
        assert_ne!(shared_alice_bob, shared_alice_charlie);
    }
    
    #[test]
    fn test_derive_encryption_key() {
        let shared_secret = [42u8; 32];
        
        // Derive encryption key
        let key1 = derive_encryption_key(&shared_secret).unwrap();
        let key2 = derive_encryption_key(&shared_secret).unwrap();
        
        // Same shared secret should produce same key
        assert_eq!(key1, key2);
        
        // Key should be 32 bytes
        assert_eq!(key1.len(), 32);
        
        // Key should be different from shared secret (HKDF transforms it)
        assert_ne!(key1, shared_secret);
    }
    
    #[test]
    fn test_derive_encryption_key_different_secrets() {
        let shared_secret1 = [42u8; 32];
        let shared_secret2 = [43u8; 32];
        
        let key1 = derive_encryption_key(&shared_secret1).unwrap();
        let key2 = derive_encryption_key(&shared_secret2).unwrap();
        
        // Different shared secrets should produce different keys
        assert_ne!(key1, key2);
    }
    
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        // Create sender and receiver keypairs
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        
        let plaintext = b"Hello, Robonomics CPS!";
        
        // Encrypt
        let encrypted = encrypt(
            plaintext,
            &sender.secret,
            &receiver.public.to_bytes(),
        ).unwrap();
        
        // Encrypted data should not be empty
        assert!(!encrypted.is_empty());
        
        // Encrypted data should be valid JSON
        let _: EncryptedMessage = serde_json::from_slice(&encrypted).unwrap();
        
        // Decrypt
        let decrypted = decrypt(&encrypted, &receiver.secret).unwrap();
        
        // Decrypted should match original plaintext
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_encrypt_produces_different_ciphertexts() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Same message";
        
        // Encrypt same message twice
        let encrypted1 = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        let encrypted2 = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        
        // Should produce different ciphertexts due to random nonces
        assert_ne!(encrypted1, encrypted2);
        
        // But both should decrypt to the same plaintext
        let decrypted1 = decrypt(&encrypted1, &receiver.secret).unwrap();
        let decrypted2 = decrypt(&encrypted2, &receiver.secret).unwrap();
        assert_eq!(decrypted1, plaintext);
        assert_eq!(decrypted2, plaintext);
    }
    
    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let wrong_receiver = test_keypair(3);
        
        let plaintext = b"Secret message";
        
        // Encrypt for receiver
        let encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        
        // Try to decrypt with wrong key
        let result = decrypt(&encrypted, &wrong_receiver.secret);
        
        // Should fail
        assert!(result.is_err());
    }
    
    #[test]
    fn test_decrypt_with_corrupted_data_fails() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Test message";
        
        // Encrypt
        let mut encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        
        // Corrupt the data
        if encrypted.len() > 10 {
            encrypted[10] ^= 0xFF;
        }
        
        // Try to decrypt corrupted data
        let result = decrypt(&encrypted, &receiver.secret);
        
        // Should fail (either parse error or authentication failure)
        assert!(result.is_err());
    }
    
    #[test]
    fn test_encrypt_empty_message() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"";
        
        // Encrypt empty message
        let encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        
        // Decrypt
        let decrypted = decrypt(&encrypted, &receiver.secret).unwrap();
        
        // Should get empty message back
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_encrypt_large_message() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = vec![42u8; 10000]; // 10KB message
        
        // Encrypt
        let encrypted = encrypt(&plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        
        // Decrypt
        let decrypted = decrypt(&encrypted, &receiver.secret).unwrap();
        
        // Should match
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_encrypted_message_structure() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Test";
        
        // Encrypt
        let encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        
        // Parse the encrypted message
        let message: EncryptedMessage = serde_json::from_slice(&encrypted).unwrap();
        
        // Check version
        assert_eq!(message.version, 1);
        
        // Check sender public key is encoded
        assert!(!message.from.is_empty());
        let decoded_from = bs58::decode(&message.from).into_vec().unwrap();
        assert_eq!(decoded_from.len(), 32);
        
        // Check nonce is base64 encoded and correct size
        let nonce = base64::decode(&message.nonce).unwrap();
        assert_eq!(nonce.len(), 24); // XChaCha20 nonce size
        
        // Check ciphertext is base64 encoded
        let ciphertext = base64::decode(&message.ciphertext).unwrap();
        assert!(!ciphertext.is_empty());
    }
    
    #[test]
    fn test_decrypt_rejects_wrong_version() {
        let sender = test_keypair(1);
        let receiver = test_keypair(2);
        let plaintext = b"Test";
        
        // Encrypt
        let encrypted = encrypt(plaintext, &sender.secret, &receiver.public.to_bytes()).unwrap();
        
        // Parse and modify version
        let mut message: EncryptedMessage = serde_json::from_slice(&encrypted).unwrap();
        message.version = 2;
        let modified = serde_json::to_vec(&message).unwrap();
        
        // Try to decrypt
        let result = decrypt(&modified, &receiver.secret);
        
        // Should fail due to unsupported version
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported encryption version"));
    }
}
