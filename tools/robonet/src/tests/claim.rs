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
use hex_literal::hex;
use libsecp256k1::{Message, PublicKey, SecretKey};
use robonomics_runtime_subxt_api::{
    api::{
        self, claim_xrt,
        runtime_types::{
            pallet_robonomics_claim::{EcdsaSignature, EthereumAddress},
            robonomics_runtime::RuntimeCall,
        },
    },
    RobonomicsConfig,
};
use sp_core::{hashing::keccak_256, H160};
use subxt::OnlineClient;
use subxt_signer::sr25519::dev;

use crate::cli::NetworkTopology;
use crate::network::NetworkEndpoints;

// Test Ethereum account with predefined seed and address
// Seed: 0xbd9880d2511d8ce62bf3d11b6e6b71b06f306d09111f21c7a625394d3b06c9df
// Private key derived from seed
// Public address: 0x1FA7503931c8dC92F25269DE61f35d0558DD6Dcc
const TEST_ETH_ADDRESS: [u8; 20] = hex!["1FA7503931c8dC92F25269DE61f35d0558DD6Dcc"];
const TEST_ETH_SEED: [u8; 32] =
    hex!["bd9880d2511d8ce62bf3d11b6e6b71b06f306d09111f21c7a625394d3b06c9df"];

/// Test: Pallet setup - add claims and fund pallet account
async fn test_pallet_setup(client: &OnlineClient<RobonomicsConfig>) -> Result<()> {
    log::info!("Testing claim pallet setup");

    // Get Alice (sudo) signer
    let signer = dev::alice();

    // Use test Ethereum account with predefined address
    let eth_address = EthereumAddress(TEST_ETH_ADDRESS);
    let claim_amount = 1_000_000_000_000u128; // 1 token with 12 decimals

    log::info!("Setting up claim for Ethereum address: {:?}", eth_address);
    log::info!("Claim amount: {}", claim_amount);

    // Get the balance of the claims pallet account before funding
    // The pallet account ID can be computed from the pallet's module prefix
    // For robonomics-claim pallet, it's typically "py/claim"

    // Add claim via sudo
    let add_claim_call = RuntimeCall::ClaimXRT(claim_xrt::Call::add_claim {
        who: eth_address.clone(),
        value: claim_amount,
    });
    let sudo_add_claim = api::tx().sudo().sudo(add_claim_call);

    let claim_events = client
        .tx()
        .sign_and_submit_then_watch_default(&sudo_add_claim, &signer)
        .await
        .context("Failed to submit add_claim transaction")?
        .wait_for_finalized_success()
        .await
        .context("add_claim transaction failed")?;

    log::info!("Added claim via sudo: {:?}", claim_events.extrinsic_hash());

    // Verify claim in storage
    let claims_query = api::storage().claim_xrt().claims(eth_address.into());
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

    // Create Ethereum keypair using test seed
    let secret_key =
        SecretKey::parse(&TEST_ETH_SEED).context("Failed to create secret key from test seed")?;
    let public_key = PublicKey::from_secret_key(&secret_key);

    // Derive Ethereum address from public key
    let public_key_bytes = &public_key.serialize()[1..]; // Skip 0x04 prefix
    let eth_address_bytes = &keccak_256(public_key_bytes)[12..]; // Take last 20 bytes
    let eth_address = H160::from_slice(eth_address_bytes);

    // Derived address should match constant
    anyhow::ensure!(
        TEST_ETH_ADDRESS == eth_address.0,
        "Derived Ethereum address does not match TEST_ETH_ADDRESS constant"
    );

    // Destination Substrate account
    let bob = dev::bob();
    let dest_account = bob.public_key().to_account_id();

    log::info!("Destination account: {:?}", dest_account);

    // Construct message to sign (following Ethereum personal_sign format)
    let prefix = b"Pay RWS to the Robonomics account:";
    let account_hex = hex::encode(dest_account.clone());
    let message = format!("{}{}", String::from_utf8_lossy(prefix), account_hex);

    log::debug!("Message to sign: {}", message);

    // Hash message with Ethereum prefix
    let eth_prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    let full_message = format!("{}{}", eth_prefix, message);
    let message_hash = keccak_256(full_message.as_bytes());

    log::debug!("Message hash: {}", hex::encode(&message_hash));

    // Sign message
    let message_to_sign = Message::parse(&message_hash);
    let (signature, recovery_id) = libsecp256k1::sign(&message_to_sign, &secret_key);

    // Construct Ethereum signature (r, s, v) - 65 bytes total
    let mut eth_signature = [0u8; 65];
    eth_signature[..64].copy_from_slice(&signature.serialize());
    eth_signature[64] = recovery_id.serialize() + 27; // Add 27 to recovery ID for Ethereum

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
    let claim_tx = api::tx()
        .claim_xrt()
        .claim(dest_account.clone(), EcdsaSignature(eth_signature));

    let claim_events = client
        .tx()
        .sign_and_submit_then_watch_default(&claim_tx, &dev::alice())
        .await
        .context("Failed to submit claim transaction")?
        .wait_for_finalized_success()
        .await
        .context("Claim transaction failed")?;

    log::info!("Claim submitted: {:?}", claim_events.extrinsic_hash());

    // Find the Claimed event
    let claimed_event = claim_events
        .find_first::<claim_xrt::events::Claimed>()
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
    let claims_query = api::storage()
        .claim_xrt()
        .claims(EthereumAddress(eth_address.0));
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
        log::info!(
            "✓ Balance increased by {} units",
            balance_after - balance_before
        );
    } else {
        anyhow::bail!("Balance did not increase after claim");
    }

    Ok(())
}

/// Test: Claim pallet functionality
pub async fn test_claim_pallet(topology: &NetworkTopology) -> Result<()> {
    log::debug!("Starting Claim pallet tests");

    let endpoints: NetworkEndpoints = topology.into();
    let client = OnlineClient::<RobonomicsConfig>::from_url(&endpoints.collator_ws)
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
