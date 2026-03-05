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
//! Basic network functionality tests.

use crate::network::NetworkClient;
use anyhow::{Context, Result};
use robonomics_runtime_subxt_api::api;
use std::time::Duration;
use subxt_signer::sr25519::dev;
use zombienet_sdk::{LocalFileSystem, Network};

/// Test: Network initialization and connectivity
pub async fn test_network_initialization(network: Option<&Network<LocalFileSystem>>) -> Result<()> {
    // Connect to relay chain via alice node
    let _ = NetworkClient::relay(network).await?;
    log::debug!("Connected to relay chain");

    // Connect to parachain collator
    let _ = NetworkClient::robonomics(network).await?;
    log::debug!("Connected to robonomics parachain");

    // Connect to AssetHub if present
    let _ = NetworkClient::assethub(network).await?;
    log::debug!("Connected to AssetHub");

    Ok(())
}

/// Test: Block production on both chains
pub async fn test_block_production(network: Option<&Network<LocalFileSystem>>) -> Result<()> {
    let relay_client = NetworkClient::relay(network).await?;

    let block1 = relay_client.blocks().at_latest().await?;
    let block_num1 = block1.number();
    log::debug!("Relay chain block: {}", block_num1);

    tokio::time::sleep(Duration::from_secs(10)).await;

    let block2 = relay_client.blocks().at_latest().await?;
    let block_num2 = block2.number();
    log::debug!("Relay chain new block: {}", block_num2);

    if block_num2 <= block_num1 {
        anyhow::bail!("Relay chain is not producing blocks");
    }

    // Check parachain
    let para_client = NetworkClient::robonomics(network).await?;

    let para_block1 = para_client.blocks().at_latest().await?;
    let para_block_num1 = para_block1.number();
    log::debug!("Parachain block: {}", para_block_num1);

    tokio::time::sleep(Duration::from_secs(20)).await;

    let para_block2 = para_client.blocks().at_latest().await?;
    let para_block_num2 = para_block2.number();
    log::debug!("Parachain new block: {}", para_block_num2);

    if para_block_num2 <= para_block_num1 {
        anyhow::bail!("Parachain is not producing blocks");
    }

    Ok(())
}

/// Test: Basic extrinsic submission
pub async fn test_extrinsic_submission(network: Option<&Network<LocalFileSystem>>) -> Result<()> {
    // Get collator node
    let client = NetworkClient::robonomics(network).await?;

    let alice = dev::alice();
    log::debug!(
        "Using Alice account: {}",
        alice.public_key().to_account_id()
    );

    // Create a remark transaction using the generated API
    let remark_tx = api::tx()
        .system()
        .remark(b"Robonet integration test".to_vec());

    // Submit and watch for finalization
    let events = client
        .tx()
        .sign_and_submit_then_watch_default(&remark_tx, &alice)
        .await
        .context("Failed to submit transaction")?
        .wait_for_finalized_success()
        .await
        .context("Transaction failed")?;

    log::info!(
        "✓ Remark transaction finalized: {:?}",
        events.extrinsic_hash()
    );

    Ok(())
}
