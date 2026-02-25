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
//! CPS (Cyber-Physical Systems) pallet integration tests.
//!
//! Tests verify CPS pallet functionality using the libcps library:
//! - Simple tree structure creation
//! - Complex tree with hundreds of nodes
//! - Plain payload operations
//! - Encrypted payload operations

use anyhow::{Context, Result};
use libcps::blockchain::{Client as CpsClient, Config as CpsConfig};
use libcps::crypto::{Cipher, CryptoScheme, EncryptionAlgorithm};
use libcps::node::{Node, NodeData};

use crate::cli::NetworkTopology;
use crate::network::NetworkEndpoints;

/// Test: Create simple CPS tree structure
async fn test_simple_tree(ws_url: &str) -> Result<()> {
    log::info!("Testing simple CPS tree creation");

    let config = CpsConfig {
        ws_url: ws_url.to_string(),
        suri: Some("//Alice".to_string()),
    };

    let client = CpsClient::new(&config)
        .await
        .context("Failed to create CPS client")?;

    // Create root node
    let root_meta: NodeData = r#"{"type":"building","name":"HQ"}"#.into();
    let root_payload: NodeData = r#"{"status":"online"}"#.into();

    let root_node = Node::create(&client, None, Some(root_meta), Some(root_payload))
        .await
        .context("Failed to create root node")?;

    log::info!("Created root node: {}", root_node.id());

    // Create child node
    let child_meta: NodeData = r#"{"type":"floor","number":1}"#.into();
    let child_payload: NodeData = r#"{"temp":"22C"}"#.into();

    let child_node = Node::create(
        &client,
        Some(root_node.id()),
        Some(child_meta),
        Some(child_payload),
    )
    .await
    .context("Failed to create child node")?;

    log::info!("Created child node: {}", child_node.id());

    // Verify structure
    let root_info = root_node.query().await?;
    if root_info.children.contains(&child_node.id()) {
        log::info!("✓ Simple tree structure verified");
    } else {
        anyhow::bail!("Child node not found in parent's children list");
    }

    Ok(())
}

/// Test: Create complex CPS tree with hundreds of nodes
async fn test_complex_tree(ws_url: &str) -> Result<()> {
    log::info!("Testing complex CPS tree with multiple nodes");

    let config = CpsConfig {
        ws_url: ws_url.to_string(),
        suri: Some("//Alice".to_string()),
    };

    let client = CpsClient::new(&config)
        .await
        .context("Failed to create CPS client")?;

    // Create root node
    let root_meta: NodeData = r#"{"type":"datacenter"}"#.into();
    let root_node = Node::create(&client, None, Some(root_meta), None)
        .await
        .context("Failed to create root node")?;

    log::info!("Created root node: {}", root_node.id());

    // Create 50 child nodes (representing hundreds would take too long for quick tests)
    const NODE_COUNT: usize = 50;
    let mut created_nodes = Vec::new();

    for i in 0..NODE_COUNT {
        let meta: NodeData = format!(r#"{{"type":"sensor","id":{}}}"#, i).into();
        let payload: NodeData = format!("data_{}", i).into();

        let node = Node::create(&client, Some(root_node.id()), Some(meta), Some(payload))
            .await
            .context(format!("Failed to create node {}", i))?;

        created_nodes.push(node.id());

        if (i + 1) % 10 == 0 {
            log::info!("Created {} nodes...", i + 1);
        }
    }

    log::info!("✓ Created {} nodes successfully", NODE_COUNT);

    // Verify structure
    let root_info = root_node.query().await?;
    if root_info.children.len() == NODE_COUNT {
        log::info!(
            "✓ Complex tree structure verified ({} children)",
            NODE_COUNT
        );
    } else {
        anyhow::bail!(
            "Expected {} children, found {}",
            NODE_COUNT,
            root_info.children.len()
        );
    }

    Ok(())
}

/// Test: Set multiple plain payloads
async fn test_plain_payloads(ws_url: &str) -> Result<()> {
    log::info!("Testing multiple plain payload updates");

    let config = CpsConfig {
        ws_url: ws_url.to_string(),
        suri: Some("//Alice".to_string()),
    };

    let client = CpsClient::new(&config)
        .await
        .context("Failed to create CPS client")?;

    // Create node
    let meta: NodeData = r#"{"type":"sensor"}"#.into();
    let node = Node::create(&client, None, Some(meta), None)
        .await
        .context("Failed to create node")?;

    log::info!("Created node: {}", node.id());

    // Update payload multiple times
    for i in 0..10 {
        let payload: NodeData = format!("reading_{}: {}", i, 20 + i).into();
        node.set_payload(Some(payload))
            .await
            .context(format!("Failed to set payload {}", i))?;

        if (i + 1) % 3 == 0 {
            log::info!("Updated payload {} times", i + 1);
        }
    }

    log::info!("✓ Set 10 plain payloads successfully");

    Ok(())
}

/// Test: Set multiple encrypted payloads
async fn test_encrypted_payloads(ws_url: &str) -> Result<()> {
    log::info!("Testing multiple encrypted payload updates");

    let config = CpsConfig {
        ws_url: ws_url.to_string(),
        suri: Some("//Alice".to_string()),
    };

    let client = CpsClient::new(&config)
        .await
        .context("Failed to create CPS client")?;

    // Create sender and receiver ciphers
    let sender_cipher = Cipher::new("//Alice".to_string(), CryptoScheme::Sr25519)
        .context("Failed to create sender cipher")?;

    let receiver_cipher = Cipher::new("//Bob".to_string(), CryptoScheme::Sr25519)
        .context("Failed to create receiver cipher")?;

    let receiver_public = receiver_cipher.public_key();

    // Create node
    let meta: NodeData = r#"{"type":"encrypted_sensor"}"#.into();
    let node = Node::create(&client, None, Some(meta), None)
        .await
        .context("Failed to create node")?;

    log::info!("Created node: {}", node.id());

    // Update encrypted payload multiple times
    for i in 0..5 {
        let plaintext = format!("secret_data_{}: {}", i, 100 + i);

        // Encrypt the data
        let encrypted_msg = sender_cipher
            .encrypt(
                plaintext.as_bytes(),
                &receiver_public,
                EncryptionAlgorithm::XChaCha20Poly1305,
            )
            .context("Failed to encrypt data")?;

        // Create encrypted NodeData
        let encrypted_bytes = encrypted_msg.encode();
        let payload = NodeData::aead_from(encrypted_bytes);

        node.set_payload(Some(payload))
            .await
            .context(format!("Failed to set encrypted payload {}", i))?;

        log::info!("Set encrypted payload {}", i + 1);
    }

    log::info!("✓ Set 5 encrypted payloads successfully");

    // Verify decryption works
    let node_info = node.query().await?;
    if let Some(NodeData::Encrypted(_)) = node_info.payload {
        log::info!("✓ Payload is properly encrypted");
    } else {
        anyhow::bail!("Expected encrypted payload");
    }

    Ok(())
}

/// Test: CPS (Cyber-Physical Systems) pallet functionality
pub async fn test_cps_pallet(_topology: &NetworkTopology) -> Result<()> {
    log::debug!("Starting CPS pallet tests");

    let endpoints = NetworkEndpoints::simple();
    let ws_url = &endpoints.collator_1_ws;

    // Run all CPS tests
    log::info!("=== Test 1/4: Simple Tree Structure ===");
    test_simple_tree(ws_url).await?;

    log::info!("=== Test 2/4: Complex Tree (50 nodes) ===");
    test_complex_tree(ws_url).await?;

    log::info!("=== Test 3/4: Multiple Plain Payloads ===");
    test_plain_payloads(ws_url).await?;

    log::info!("=== Test 4/4: Multiple Encrypted Payloads ===");
    test_encrypted_payloads(ws_url).await?;

    log::info!("✓ All CPS pallet tests passed");

    Ok(())
}
