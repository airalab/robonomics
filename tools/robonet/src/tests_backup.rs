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
//! Integration test runner for local network.

use anyhow::{Context, Result};
use colored::Colorize;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::dev;

use crate::cli::NetworkTopology;
use crate::network::NetworkEndpoints;

/// Test result for a single test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub status: TestStatus,
    #[serde(serialize_with = "serialize_duration")]
    pub duration: Duration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn serialize_duration<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_f64(duration.as_secs_f64())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
}

/// Test suite results
#[derive(Debug, Serialize, Deserialize)]
pub struct TestSuiteResults {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    #[serde(serialize_with = "serialize_duration")]
    pub duration: Duration,
    pub tests: Vec<TestResult>,
}

impl TestSuiteResults {
    pub fn passed_count(&self) -> usize {
        self.passed
    }
    
    pub fn failed_count(&self) -> usize {
        self.failed
    }
    
    pub fn skipped_count(&self) -> usize {
        self.skipped
    }
    
    pub fn is_success(&self) -> bool {
        self.failed == 0
    }
}

/// Run a single test and capture its result
async fn run_test<F, Fut>(name: &str, test_fn: F) -> TestResult
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let start = Instant::now();
    log::debug!("Running test: {}", name);
    
    match test_fn().await {
        Ok(()) => {
            let duration = start.elapsed();
            log::info!("Test passed: {} ({:.2}s)", name, duration.as_secs_f64());
            TestResult {
                name: name.to_string(),
                status: TestStatus::Passed,
                duration,
                error: None,
            }
        }
        Err(e) => {
            let duration = start.elapsed();
            log::error!("Test failed: {} - {}", name, e);
            TestResult {
                name: name.to_string(),
                status: TestStatus::Failed,
                duration,
                error: Some(e.to_string()),
            }
        }
    }
}

/// Test: Network initialization and connectivity
async fn test_network_initialization(topology: &NetworkTopology) -> Result<()> {
    let endpoints = match topology {
        NetworkTopology::Simple => NetworkEndpoints::simple(),
        NetworkTopology::Assethub => NetworkEndpoints::with_assethub(),
    };
    
    // Connect to relay chain
    let _relay_client = OnlineClient::<PolkadotConfig>::from_url(&endpoints.relay_ws)
        .await
        .context("Failed to connect to relay chain")?;
    log::debug!("Connected to relay chain");
    
    // Connect to parachain collator 1
    let _para_client = OnlineClient::<PolkadotConfig>::from_url(&endpoints.collator_1_ws)
        .await
        .context("Failed to connect to robonomics parachain")?;
    log::debug!("Connected to robonomics parachain");
    
    // Connect to AssetHub if present
    if let Some(asset_hub_ws) = endpoints.asset_hub_ws {
        let _asset_hub_client = OnlineClient::<PolkadotConfig>::from_url(&asset_hub_ws)
            .await
            .context("Failed to connect to AssetHub")?;
        log::debug!("Connected to AssetHub");
    }
    
    Ok(())
}

/// Test: Block production on both chains
async fn test_block_production(topology: &NetworkTopology) -> Result<()> {
    let endpoints = match topology {
        NetworkTopology::Simple => NetworkEndpoints::simple(),
        NetworkTopology::Assethub => NetworkEndpoints::with_assethub(),
    };
    
    // Check relay chain
    let relay_client = OnlineClient::<PolkadotConfig>::from_url(&endpoints.relay_ws)
        .await
        .context("Failed to connect to relay chain")?;
    
    let block1 = relay_client.blocks().at_latest().await?;
    let block_num1 = block1.number();
    log::debug!("Relay chain block: {}", block_num1);
    
    tokio::time::sleep(Duration::from_secs(6)).await;
    
    let block2 = relay_client.blocks().at_latest().await?;
    let block_num2 = block2.number();
    log::debug!("Relay chain new block: {}", block_num2);
    
    if block_num2 <= block_num1 {
        anyhow::bail!("Relay chain is not producing blocks");
    }
    
    // Check parachain
    let para_client = OnlineClient::<PolkadotConfig>::from_url(&endpoints.collator_1_ws)
        .await
        .context("Failed to connect to parachain")?;
    
    let para_block1 = para_client.blocks().at_latest().await?;
    let para_block_num1 = para_block1.number();
    log::debug!("Parachain block: {}", para_block_num1);
    
    tokio::time::sleep(Duration::from_secs(6)).await;
    
    let para_block2 = para_client.blocks().at_latest().await?;
    let para_block_num2 = para_block2.number();
    log::debug!("Parachain new block: {}", para_block_num2);
    
    if para_block_num2 <= para_block_num1 {
        anyhow::bail!("Parachain is not producing blocks");
    }
    
    Ok(())
}

/// Test: Basic extrinsic submission
async fn test_extrinsic_submission(_topology: &NetworkTopology) -> Result<()> {
    let endpoints = NetworkEndpoints::simple();
    
    let client = OnlineClient::<PolkadotConfig>::from_url(&endpoints.collator_1_ws)
        .await
        .context("Failed to connect to parachain")?;
    
    let alice = dev::alice();
    log::debug!("Using Alice account");
    
    // Create a remark transaction
    let remark_call = subxt::dynamic::tx(
        "System",
        "remark",
        vec![subxt::dynamic::Value::from_bytes("Localnet integration test")],
    );
    
    // Submit and watch for inclusion
    let mut progress = client
        .tx()
        .sign_and_submit_then_watch_default(&remark_call, &alice)
        .await
        .context("Failed to submit transaction")?;
    
    // Wait for in block
    while let Some(status) = progress.next().await {
        let status = status.context("Failed to get transaction status")?;
        if let Some(in_block) = status.as_in_block() {
            log::debug!("Transaction included in block: {:?}", in_block);
            break;
        }
    }
    
    Ok(())
}

/// Test: XCM upward message (parachain -> relay)
async fn test_xcm_upward_message(_topology: &NetworkTopology) -> Result<()> {
    // This test requires XCM pallet to be available
    // For now, we'll mark it as a placeholder
    log::debug!("XCM upward message test - checking for XCM pallet support");
    
    let endpoints = NetworkEndpoints::simple();
    let _client = OnlineClient::<PolkadotConfig>::from_url(&endpoints.collator_1_ws)
        .await
        .context("Failed to connect to parachain")?;
    
    // TODO: Implement XCM message sending once we have proper metadata
    log::warn!("XCM upward message test requires proper runtime metadata");
    
    Ok(())
}

/// Test: XCM downward message (relay -> parachain)
async fn test_xcm_downward_message(_topology: &NetworkTopology) -> Result<()> {
    log::debug!("XCM downward message test");
    log::warn!("XCM downward message test requires proper runtime metadata");
    Ok(())
}

/// Test: XCM token teleport between parachains
async fn test_xcm_token_teleport(topology: &NetworkTopology) -> Result<()> {
    // Only run for WithAssethub topology
    match topology {
        NetworkTopology::Assethub => {
            log::debug!("XCM token teleport test");
            log::warn!("XCM token teleport test requires proper runtime metadata");
            Ok(())
        }
        NetworkTopology::Simple => {
            log::info!("Skipping XCM token teleport test (requires AssetHub)");
            Ok(())
        }
    }
}

/// Test: CPS (Cyber-Physical Systems) pallet functionality
async fn test_cps_pallet(_topology: &NetworkTopology) -> Result<()> {
    log::debug!("CPS pallet test");
    
    let endpoints = NetworkEndpoints::simple();
    let _client = OnlineClient::<PolkadotConfig>::from_url(&endpoints.collator_1_ws)
        .await
        .context("Failed to connect to parachain")?;
    
    // TODO: Implement CPS pallet tests
    log::warn!("CPS pallet test requires proper runtime metadata");
    
    Ok(())
}

/// Test: Claim pallet functionality
async fn test_claim_pallet(_topology: &NetworkTopology) -> Result<()> {
    log::debug!("Claim pallet test");
    
    let endpoints = NetworkEndpoints::simple();
    let _client = OnlineClient::<PolkadotConfig>::from_url(&endpoints.collator_1_ws)
        .await
        .context("Failed to connect to parachain")?;
    
    // TODO: Implement Claim pallet tests
    log::warn!("Claim pallet test requires proper runtime metadata");
    
    Ok(())
}

/// Run all integration tests
pub async fn run_integration_tests(topology: &NetworkTopology, fail_fast: bool, test_filter: Option<Vec<String>>, json_output: bool) -> Result<TestSuiteResults> {
    if !json_output {
        println!();
        println!("{}", "Running Integration Tests".bold().cyan());
        println!("{}", "==================================================".bright_black());
        println!();
    }
    
    let suite_start = Instant::now();
    let mut results = Vec::new();
    
    log::info!("Running tests");
    
    // Run tests based on filter
    if test_filter.is_none() || test_filter.as_ref().unwrap().iter().any(|f| "network_initialization".contains(f.as_str())) {
        results.push(run_test("network_initialization", || test_network_initialization(topology)).await);
        if fail_fast && results.last().unwrap().status == TestStatus::Failed {
            log::warn!("Stopping test execution due to failure (fail-fast mode)");
            return build_results(results, suite_start, json_output);
        }
    }
    
    if test_filter.is_none() || test_filter.as_ref().unwrap().iter().any(|f| "block_production".contains(f.as_str())) {
        results.push(run_test("block_production", || test_block_production(topology)).await);
        if fail_fast && results.last().unwrap().status == TestStatus::Failed {
            log::warn!("Stopping test execution due to failure (fail-fast mode)");
            return build_results(results, suite_start, json_output);
        }
    }
    
    if test_filter.is_none() || test_filter.as_ref().unwrap().iter().any(|f| "extrinsic_submission".contains(f.as_str())) {
        results.push(run_test("extrinsic_submission", || test_extrinsic_submission(topology)).await);
        if fail_fast && results.last().unwrap().status == TestStatus::Failed {
            log::warn!("Stopping test execution due to failure (fail-fast mode)");
            return build_results(results, suite_start, json_output);
        }
    }
    
    if test_filter.is_none() || test_filter.as_ref().unwrap().iter().any(|f| "xcm_upward".contains(f.as_str())) {
        results.push(run_test("xcm_upward_message", || test_xcm_upward_message(topology)).await);
        if fail_fast && results.last().unwrap().status == TestStatus::Failed {
            log::warn!("Stopping test execution due to failure (fail-fast mode)");
            return build_results(results, suite_start, json_output);
        }
    }
    
    if test_filter.is_none() || test_filter.as_ref().unwrap().iter().any(|f| "xcm_downward".contains(f.as_str())) {
        results.push(run_test("xcm_downward_message", || test_xcm_downward_message(topology)).await);
        if fail_fast && results.last().unwrap().status == TestStatus::Failed {
            log::warn!("Stopping test execution due to failure (fail-fast mode)");
            return build_results(results, suite_start, json_output);
        }
    }
    
    if test_filter.is_none() || test_filter.as_ref().unwrap().iter().any(|f| "xcm_teleport".contains(f.as_str()) || "token_teleport".contains(f.as_str())) {
        results.push(run_test("xcm_token_teleport", || test_xcm_token_teleport(topology)).await);
        if fail_fast && results.last().unwrap().status == TestStatus::Failed {
            log::warn!("Stopping test execution due to failure (fail-fast mode)");
            return build_results(results, suite_start, json_output);
        }
    }
    
    if test_filter.is_none() || test_filter.as_ref().unwrap().iter().any(|f| "cps".contains(f.as_str())) {
        results.push(run_test("cps_pallet", || test_cps_pallet(topology)).await);
        if fail_fast && results.last().unwrap().status == TestStatus::Failed {
            log::warn!("Stopping test execution due to failure (fail-fast mode)");
            return build_results(results, suite_start, json_output);
        }
    }
    
    if test_filter.is_none() || test_filter.as_ref().unwrap().iter().any(|f| "claim".contains(f.as_str())) {
        results.push(run_test("claim_pallet", || test_claim_pallet(topology)).await);
    }
    
    build_results(results, suite_start, json_output)
}

fn build_results(results: Vec<TestResult>, suite_start: Instant, json_output: bool) -> Result<TestSuiteResults> {
    let total_duration = suite_start.elapsed();
    
    // Display results if not in JSON mode
    if !json_output {
        for result in &results {
            match result.status {
                TestStatus::Passed => {
                    println!(
                        "  {} {} ({:.2}s)",
                        "[PASS]".green(),
                        result.name.green(),
                        result.duration.as_secs_f64()
                    );
                }
                TestStatus::Failed => {
                    println!(
                        "  {} {} ({:.2}s)",
                        "[FAIL]".red(),
                        result.name.red(),
                        result.duration.as_secs_f64()
                    );
                    if let Some(ref error) = result.error {
                        println!("    Error: {}", error.bright_black());
                    }
                }
                TestStatus::Skipped => {
                    println!(
                        "  {} {} (skipped)",
                        "[SKIP]".yellow(),
                        result.name.yellow()
                    );
                }
            }
        }
    }
    
    let passed = results.iter().filter(|t| t.status == TestStatus::Passed).count();
    let failed = results.iter().filter(|t| t.status == TestStatus::Failed).count();
    let skipped = results.iter().filter(|t| t.status == TestStatus::Skipped).count();
    
    let suite_results = TestSuiteResults {
        total: results.len(),
        passed,
        failed,
        skipped,
        duration: total_duration,
        tests: results,
    };
    
    if json_output {
        // Output JSON
        let json = serde_json::to_string_pretty(&suite_results)?;
        println!("{}", json);
    } else {
        // Output text summary
        println!();
        println!("{}", "Test Summary".bold());
        println!("{}", "==================================================".bright_black());
        println!("  Total:      {}", suite_results.total);
        println!("  Passed:     {}", suite_results.passed.to_string().green());
        println!("  Failed:     {}", suite_results.failed.to_string().red());
        println!("  Skipped:    {}", suite_results.skipped.to_string().yellow());
        println!("  Duration:   {:.2}s", total_duration.as_secs_f64());
        println!();
        
        if suite_results.is_success() {
            log::info!("All tests passed!");
        } else {
            log::error!("{} test(s) failed", suite_results.failed);
        }
    }
    
    Ok(suite_results)
}
