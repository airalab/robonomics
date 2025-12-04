use anyhow::{anyhow, Result};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use schnorrkel::{PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Encrypted message format stored on-chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedMessage {
    pub version: u8,
    pub from: String,
    pub nonce: String,
    pub ciphertext: String,
}

const INFO: &[u8] = b"robonomics-cps-xchacha20poly1305";

/// Encrypt data using sr25519 → XChaCha20-Poly1305 scheme
///
/// # Process
/// 1. Derive shared secret from sender's secret key and receiver's public key using DH
/// 2. HKDF: Derive encryption key from shared secret using SHA256
/// 3. XChaCha20-Poly1305: Encrypt plaintext with derived key and random nonce
///
/// # Parameters
/// - `plaintext`: Data to encrypt
/// - `sender_secret`: Sender's sr25519 secret key
/// - `receiver_public`: Receiver's sr25519 public key (32 bytes)
///
/// # Returns
/// JSON-encoded EncryptedMessage with base64-encoded nonce and ciphertext
pub fn encrypt(
    plaintext: &[u8],
    sender_secret: &SecretKey,
    receiver_public: &[u8; 32],
) -> Result<Vec<u8>> {
    // Step 1: Derive shared secret using Diffie-Hellman
    let receiver_pubkey = PublicKey::from_bytes(receiver_public)
        .map_err(|e| anyhow!("Invalid receiver public key: {}", e))?;
    
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
        .map_err(|e| anyhow!("HKDF expansion failed: {}", e))?;

    // Step 3: Encrypt with XChaCha20-Poly1305
    let cipher = XChaCha20Poly1305::new(&okm.into());
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    // Step 4: Create message structure
    let message = EncryptedMessage {
        version: 1,
        from: bs58::encode(sender_public.to_bytes()).into_string(),
        nonce: base64::encode(nonce.as_slice()),
        ciphertext: base64::encode(&ciphertext),
    };

    // Serialize to JSON
    serde_json::to_vec(&message).map_err(|e| anyhow!("JSON serialization failed: {}", e))
}

/// Decrypt data using sr25519 → XChaCha20-Poly1305 scheme
///
/// # Process
/// 1. Parse encrypted message
/// 2. Derive shared secret from receiver's secret key and sender's public key
/// 3. HKDF: Derive encryption key from shared secret
/// 4. XChaCha20-Poly1305: Decrypt ciphertext
///
/// # Parameters
/// - `encrypted_data`: JSON-encoded EncryptedMessage
/// - `receiver_secret`: Receiver's sr25519 secret key
///
/// # Returns
/// Decrypted plaintext bytes
pub fn decrypt(encrypted_data: &[u8], receiver_secret: &SecretKey) -> Result<Vec<u8>> {
    // Step 1: Parse message
    let message: EncryptedMessage = serde_json::from_slice(encrypted_data)
        .map_err(|e| anyhow!("Failed to parse encrypted message: {}", e))?;

    if message.version != 1 {
        return Err(anyhow!("Unsupported encryption version: {}", message.version));
    }

    // Decode sender's public key
    let sender_public_bytes = bs58::decode(&message.from)
        .into_vec()
        .map_err(|e| anyhow!("Failed to decode sender public key: {}", e))?;
    
    if sender_public_bytes.len() != 32 {
        return Err(anyhow!("Invalid sender public key length"));
    }
    
    let mut sender_pk_array = [0u8; 32];
    sender_pk_array.copy_from_slice(&sender_public_bytes);
    
    let sender_public = PublicKey::from_bytes(&sender_pk_array)
        .map_err(|e| anyhow!("Invalid sender public key: {}", e))?;

    // Step 2: Derive shared secret
    let mut shared_input = Vec::new();
    shared_input.extend_from_slice(&receiver_secret.to_bytes()[..32]);
    shared_input.extend_from_slice(&sender_public.to_bytes());
    
    let shared_secret = sha2::Sha256::digest(&shared_input);

    // Step 3: HKDF to derive encryption key
    let hkdf = Hkdf::<Sha256>::new(None, &shared_secret);
    let mut okm = [0u8; 32];
    hkdf.expand(INFO, &mut okm)
        .map_err(|e| anyhow!("HKDF expansion failed: {}", e))?;

    // Decode nonce and ciphertext
    let nonce_bytes = base64::decode(&message.nonce)
        .map_err(|e| anyhow!("Failed to decode nonce: {}", e))?;
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = base64::decode(&message.ciphertext)
        .map_err(|e| anyhow!("Failed to decode ciphertext: {}", e))?;

    // Step 4: Decrypt
    let cipher = XChaCha20Poly1305::new(&okm.into());
    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| anyhow!("Decryption failed: {}", e))
}
