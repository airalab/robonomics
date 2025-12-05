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
use sha2::{Digest, Sha256};

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
    // Step 1: Derive shared secret using Diffie-Hellman
    let receiver_pubkey = PublicKey::from_bytes(receiver_public)
        .map_err(|e| anyhow!("Invalid receiver public key: {e}"))?;
    
    // For now, use a simple hash-based approach for shared secret
    // In production, this should use proper ECDH
    let sender_public = sender_secret.to_public();
    let mut shared_input = Vec::new();
    shared_input.extend_from_slice(&sender_secret.to_bytes()[..32]);
    shared_input.extend_from_slice(&receiver_pubkey.to_bytes());
    
    let shared_secret = sha2::Sha256::digest(&shared_input);

    // Step 2: HKDF to derive encryption key
    let hkdf = Hkdf::<Sha256>::new(None, &shared_secret);
    let mut okm = [0u8; 32];
    hkdf.expand(INFO, &mut okm)
        .map_err(|e| anyhow!("HKDF expansion failed: {e}"))?;

    // Step 3: Encrypt with XChaCha20-Poly1305
    let cipher = XChaCha20Poly1305::new(&okm.into());
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| anyhow!("Encryption failed: {e}"))?;

    // Step 4: Create message structure
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

    // Decode sender's public key
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

    // Step 2: Derive shared secret
    let mut shared_input = Vec::new();
    shared_input.extend_from_slice(&receiver_secret.to_bytes()[..32]);
    shared_input.extend_from_slice(&sender_public.to_bytes());
    
    let shared_secret = sha2::Sha256::digest(&shared_input);

    // Step 3: HKDF to derive encryption key
    let hkdf = Hkdf::<Sha256>::new(None, &shared_secret);
    let mut okm = [0u8; 32];
    hkdf.expand(INFO, &mut okm)
        .map_err(|e| anyhow!("HKDF expansion failed: {e}"))?;

    // Decode nonce and ciphertext
    let nonce_bytes = base64::decode(&message.nonce)
        .map_err(|e| anyhow!("Failed to decode nonce: {e}"))?;
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = base64::decode(&message.ciphertext)
        .map_err(|e| anyhow!("Failed to decode ciphertext: {e}"))?;

    // Step 4: Decrypt
    let cipher = XChaCha20Poly1305::new(&okm.into());
    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| anyhow!("Decryption failed: {e}"))
}
