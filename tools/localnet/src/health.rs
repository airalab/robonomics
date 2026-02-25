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
//! Health check utilities for network verification.

use anyhow::{Context, Result};
use colored::Colorize;
use subxt::{OnlineClient, PolkadotConfig};

use crate::network::NetworkEndpoints;

/// Health status for a single node
#[derive(Debug)]
pub struct NodeHealth {
    pub name: String,
    pub endpoint: String,
    pub is_healthy: bool,
    pub block_number: Option<u32>,
    pub error: Option<String>,
}

/// Overall network health
#[derive(Debug)]
pub struct NetworkHealth {
    pub nodes: Vec<NodeHealth>,
}

impl NetworkHealth {
    pub fn is_healthy(&self) -> bool {
        self.nodes.iter().all(|n| n.is_healthy)
    }
    
    pub fn passed_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_healthy).count()
    }
    
    pub fn failed_count(&self) -> usize {
        self.nodes.iter().filter(|n| !n.is_healthy).count()
    }
}

/// Check if a node is healthy
async fn check_node_health(name: &str, endpoint: &str) -> NodeHealth {
    log::debug!("Checking health of {} at {}", name, endpoint);
    
    match OnlineClient::<PolkadotConfig>::from_url(endpoint).await {
        Ok(client) => {
            match client.blocks().at_latest().await {
                Ok(block) => {
                    let block_number = block.number();
                    log::debug!("{} is healthy at block #{}", name, block_number);
                    NodeHealth {
                        name: name.to_string(),
                        endpoint: endpoint.to_string(),
                        is_healthy: true,
                        block_number: Some(block_number),
                        error: None,
                    }
                }
                Err(e) => {
                    log::warn!("{} connection ok but failed to fetch block: {}", name, e);
                    NodeHealth {
                        name: name.to_string(),
                        endpoint: endpoint.to_string(),
                        is_healthy: false,
                        block_number: None,
                        error: Some(format!("Failed to fetch block: {}", e)),
                    }
                }
            }
        }
        Err(e) => {
            log::warn!("{} is not reachable: {}", name, e);
            NodeHealth {
                name: name.to_string(),
                endpoint: endpoint.to_string(),
                is_healthy: false,
                block_number: None,
                error: Some(format!("Connection failed: {}", e)),
            }
        }
    }
}

/// Perform health checks on all network nodes
pub async fn check_network_health(detailed: bool) -> Result<NetworkHealth> {
    use colored::Colorize;
    
    println!();
    println!("{}", "🏥 Network Health Check".bold().cyan());
    println!("{}", "━".repeat(50).bright_black());
    println!();
    
    let endpoints = NetworkEndpoints::new();
    
    let nodes = vec![
        ("Relay Chain", endpoints.relay_ws.as_str()),
        ("Collator 1", endpoints.collator_1_ws.as_str()),
        ("Collator 2", endpoints.collator_2_ws.as_str()),
    ];
    
    let mut health_results = Vec::new();
    
    for (name, endpoint) in nodes {
        let health = check_node_health(name, endpoint).await;
        
        // Display result
        if health.is_healthy {
            print!("  {} {:<20}", "✓".green(), name.green());
            if let Some(block_num) = health.block_number {
                println!("Block #{}", block_num.to_string().bright_black());
            } else {
                println!();
            }
            
            if detailed {
                println!("    {}: {}", "Endpoint".bright_black(), endpoint.bright_black());
            }
        } else {
            print!("  {} {:<20}", "✗".red(), name.red());
            if let Some(ref error) = health.error {
                println!("{}", error.bright_black());
            } else {
                println!();
            }
            
            if detailed {
                println!("    {}: {}", "Endpoint".bright_black(), endpoint.bright_black());
            }
        }
        
        health_results.push(health);
    }
    
    println!();
    let network_health = NetworkHealth { nodes: health_results };
    
    // Display summary
    println!("{}", "Summary".bold());
    println!("{}", "━".repeat(50).bright_black());
    println!("  Total nodes:    {}", nodes.len());
    println!("  Healthy:        {}", network_health.passed_count().to_string().green());
    println!("  Unhealthy:      {}", network_health.failed_count().to_string().red());
    println!();
    
    if network_health.is_healthy() {
        log::info!("All network nodes are healthy");
    } else {
        log::warn!("{} out of {} nodes are unhealthy", 
            network_health.failed_count(), 
            network_health.nodes.len());
    }
    
    Ok(network_health)
}

/// Verify blocks are being produced
pub async fn verify_block_production(endpoint: &str, wait_secs: u64) -> Result<bool> {
    let client = OnlineClient::<PolkadotConfig>::from_url(endpoint)
        .await
        .context("Failed to connect to node")?;
    
    let block1 = client.blocks().at_latest().await?;
    let block_num1 = block1.number();
    
    log::debug!("Initial block number: {}", block_num1);
    
    tokio::time::sleep(tokio::time::Duration::from_secs(wait_secs)).await;
    
    let block2 = client.blocks().at_latest().await?;
    let block_num2 = block2.number();
    
    log::debug!("New block number: {}", block_num2);
    
    let is_producing = block_num2 > block_num1;
    
    if is_producing {
        log::debug!("Block production verified: {} -> {}", block_num1, block_num2);
    } else {
        log::warn!("No new blocks produced: {} -> {}", block_num1, block_num2);
    }
    
    Ok(is_producing)
}
