///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2026 Robonomics Network <research@robonomics.network>
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
//! Claim pallet integration tests.
//!
//! Tests verify Claim pallet functionality:
//! - Pallet setup when chain is live
//! - Token claiming from Ethereum accounts

use anyhow::{Context, Result};
use robonomics_runtime_subxt_api::{api, RobonomicsConfig};
use sp_core::{Pair, H160};
use sp_runtime::AccountId32;
use subxt::{tx::PairSigner, OnlineClient};
use subxt_signer::sr25519::dev;

use crate::cli::NetworkTopology;
use crate::network::NetworkEndpoints;

// Ethereum address type (20 bytes)
type EthereumAddress = H160;

// Test Ethereum account with predefined seed and address
// Seed: 0x0000000000000000000000000000000000000000000000000000000000000001
// Private key derived from seed
// Public address: 0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf
const TEST_ETH_ADDRESS: &str = "0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf";
const TEST_ETH_SEED: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
];

/// Test: Pallet setup - add claims and fund pallet account
async fn test_pallet_setup(client: &OnlineClient<RobonomicsConfig>) -> Result<()> {
    log::info!("Testing claim pallet setup");

    // Get Alice (sudo) signer
    let alice = dev::alice();
    let signer = PairSigner::new(alice);

    // Use test Ethereum account with predefined address
    let eth_address: EthereumAddress = TEST_ETH_ADDRESS
        .parse()
        .context("Failed to parse test Ethereum address")?;
    let claim_amount = 1_000_000_000_000u128; // 1 token with 12 decimals

    log::info!(
        "Setting up claim for Ethereum address: {}",
        TEST_ETH_ADDRESS
    );
    log::info!("Claim amount: {}", claim_amount);

    // Get the balance of the claims pallet account before funding
    // The pallet account ID can be computed from the pallet's module prefix
    // For robonomics-claim pallet, it's typically "py/claim"
    
    // Add claim via sudo
    let add_claim_tx = api::tx().claim().add_claim(eth_address.into(), claim_amount);
    let sudo_add_claim = api::tx().sudo().sudo(add_claim_tx);

    let claim_events = client
        .tx()
        .sign_and_submit_then_watch_default(&sudo_add_claim, &signer)
        .await
        .context("Failed to submit add_claim transaction")?
        .wait_for_finalized_success()
        .await
        .context("add_claim transaction failed")?;

    log::info!("Added claim via sudo: block {:?}", claim_events.block_hash());

    // Verify claim in storage
    let claims_query = api::storage().claim().claims(eth_address.into());
    let stored_claim = client
        .storage()
        .at_latest()
        .await?
        .fetch(&claims_query)
        .await
        .context("Failed to fetch claim from storage")?;

    if let Some(stored_amount) = stored_claim {
        if stored_amount == claim_amount {
            log::info!("✓ Claim verified in storage: {} units", stored_amount);
        } else {
            anyhow::bail!(
                "Claim amount mismatch: expected {}, got {}",
                claim_amount,
                stored_amount
            );
        }
    } else {
        anyhow::bail!("Claim not found in storage");
    }

    Ok(())
}

/// Test: Claim tokens from Ethereum account
async fn test_claim_from_ethereum(client: &OnlineClient<RobonomicsConfig>) -> Result<()> {
    log::info!("Testing token claim from Ethereum account");

    // Generate test Ethereum key using predefined seed
    use secp256k1::{Message, Secp256k1, SecretKey};
    use sp_core::hashing::keccak_256;

    let secp = Secp256k1::new();

    // Create Ethereum keypair using test seed
    let secret_key = SecretKey::from_slice(&TEST_ETH_SEED)
        .context("Failed to create secret key from test seed")?;
    let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);

    // Derive Ethereum address from public key
    let public_key_bytes = &public_key.serialize_uncompressed()[1..]; // Skip 0x04 prefix
    let eth_address_bytes = &keccak_256(public_key_bytes)[12..]; // Take last 20 bytes
    let eth_address = EthereumAddress::from_slice(eth_address_bytes);

    log::info!("Test Ethereum address: {}", TEST_ETH_ADDRESS);
    log::info!("Derived Ethereum address: {:?}", eth_address);

    // Destination Substrate account
    let bob = dev::bob();
    let dest_account = AccountId32::from(bob.public_key().0);

    log::info!("Destination account: {:?}", dest_account);

    // Construct message to sign (following Ethereum personal_sign format)
    let prefix = b"Pay RWS to the Robonomics account:";
    let account_hex = hex::encode(dest_account.as_ref());
    let message = format!("{}{}", String::from_utf8_lossy(prefix), account_hex);

    log::debug!("Message to sign: {}", message);

    // Hash message with Ethereum prefix
    let eth_prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    let full_message = format!("{}{}", eth_prefix, message);
    let message_hash = keccak_256(full_message.as_bytes());

    log::debug!("Message hash: {}", hex::encode(&message_hash));

    // Sign with secp256k1
    let secp_message = Message::from_digest_slice(&message_hash)
        .context("Failed to create message from hash")?;
    let (recovery_id, signature_bytes) = secp
        .sign_ecdsa_recoverable(&secp_message, &secret_key)
        .serialize_compact();

    // Construct Ethereum signature (r, s, v) - 65 bytes total
    let mut eth_signature = [0u8; 65];
    eth_signature[..64].copy_from_slice(&signature_bytes);
    eth_signature[64] = recovery_id.to_i32() as u8 + 27; // Add 27 to recovery ID for Ethereum

    log::info!("Ethereum signature: {}", hex::encode(&eth_signature));

    // Get Bob's balance before claim
    let balance_query = api::storage().system().account(dest_account.clone());
    let account_before = client
        .storage()
        .at_latest()
        .await?
        .fetch(&balance_query)
        .await
        .context("Failed to fetch account")?;

    let balance_before = account_before.map(|a| a.data.free).unwrap_or(0);
    log::info!("Bob's balance before claim: {}", balance_before);

    // Submit claim transaction (anyone can submit, not just the destination account)
    let claim_tx = api::tx().claim().claim(dest_account.clone(), eth_signature);

    let claim_events = client
        .tx()
        .sign_and_submit_then_watch_default(&claim_tx, &dev::alice())
        .await
        .context("Failed to submit claim transaction")?
        .wait_for_finalized_success()
        .await
        .context("Claim transaction failed")?;

    log::info!("Claim submitted in block: {:?}", claim_events.block_hash());

    // Find the Claimed event
    let claimed_event = claim_events
        .find_first::<api::claim::events::Claimed>()
        .context("Failed to find events")?;

    if let Some(event) = claimed_event {
        log::info!(
            "✓ Claim successful: {} tokens transferred to {:?}",
            event.amount,
            event.who
        );
    } else {
        anyhow::bail!("Claimed event not found in transaction events");
    }

    // Verify claim was removed from storage
    let claims_query = api::storage().claim().claims(eth_address.into());
    let remaining_claim = client
        .storage()
        .at_latest()
        .await?
        .fetch(&claims_query)
        .await?;

    if remaining_claim.is_none() {
        log::info!("✓ Claim removed from storage after processing");
    } else {
        anyhow::bail!("Claim still exists in storage after claiming");
    }

    // Verify Bob's balance increased
    let account_after = client
        .storage()
        .at_latest()
        .await?
        .fetch(&balance_query)
        .await
        .context("Failed to fetch account after claim")?;

    let balance_after = account_after.map(|a| a.data.free).unwrap_or(0);
    log::info!("Bob's balance after claim: {}", balance_after);

    if balance_after > balance_before {
        log::info!("✓ Balance increased by {} units", balance_after - balance_before);
    } else {
        anyhow::bail!("Balance did not increase after claim");
    }

    Ok(())
}

/// Test: Claim pallet functionality
pub async fn test_claim_pallet(_topology: &NetworkTopology) -> Result<()> {
    log::debug!("Starting Claim pallet tests");

    let endpoints = NetworkEndpoints::simple();
    let client = OnlineClient::<RobonomicsConfig>::from_url(&endpoints.collator_1_ws)
        .await
        .context("Failed to connect to parachain")?;

    // Run all claim tests
    log::info!("=== Test 1/2: Pallet Setup ===");
    test_pallet_setup(&client).await?;

    log::info!("=== Test 2/2: Claim from Ethereum ===");
    test_claim_from_ethereum(&client).await?;

    log::info!("✓ All Claim pallet tests passed");

    Ok(())
}
