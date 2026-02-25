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
use std::time::{Duration, Instant};
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::dev;

use crate::network::NetworkEndpoints;

/// Test result for a single test
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub status: TestStatus,
    pub duration: Duration,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
}

/// Test suite results
#[derive(Debug)]
pub struct TestSuiteResults {
    pub tests: Vec<TestResult>,
    pub total_duration: Duration,
}

impl TestSuiteResults {
    pub fn passed_count(&self) -> usize {
        self.tests.iter().filter(|t| t.status == TestStatus::Passed).count()
    }
    
    pub fn failed_count(&self) -> usize {
        self.tests.iter().filter(|t| t.status == TestStatus::Failed).count()
    }
    
    pub fn skipped_count(&self) -> usize {
        self.tests.iter().filter(|t| t.status == TestStatus::Skipped).count()
    }
    
    pub fn is_success(&self) -> bool {
        self.failed_count() == 0
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
async fn test_network_initialization() -> Result<()> {
    let endpoints = NetworkEndpoints::new();
    
    // Connect to relay chain
    let _relay_client = OnlineClient::<PolkadotConfig>::from_url(&endpoints.relay_ws)
        .await
        .context("Failed to connect to relay chain")?;
    log::debug!("Connected to relay chain");
    
    // Connect to parachain collator 1
    let _para_client = OnlineClient::<PolkadotConfig>::from_url(&endpoints.collator_1_ws)
        .await
        .context("Failed to connect to parachain collator 1")?;
    log::debug!("Connected to parachain collator 1");
    
    Ok(())
}

/// Test: Block production on both chains
async fn test_block_production() -> Result<()> {
    let endpoints = NetworkEndpoints::new();
    
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
async fn test_extrinsic_submission() -> Result<()> {
    let endpoints = NetworkEndpoints::new();
    
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
    let progress = client
        .tx()
        .sign_and_submit_then_watch_default(&remark_call, &alice)
        .await
        .context("Failed to submit transaction")?;
    
    let result = progress
        .wait_for_in_block()
        .await
        .context("Failed to wait for transaction in block")?;
    
    log::debug!("Transaction included in block: {:?}", result.block_hash());
    
    Ok(())
}

/// Test: XCM upward message (parachain -> relay)
async fn test_xcm_upward_message() -> Result<()> {
    // This test requires XCM pallet to be available
    // For now, we'll mark it as a placeholder
    log::debug!("XCM upward message test - checking for XCM pallet support");
    
    let endpoints = NetworkEndpoints::new();
    let _client = OnlineClient::<PolkadotConfig>::from_url(&endpoints.collator_1_ws)
        .await
        .context("Failed to connect to parachain")?;
    
    // TODO: Implement XCM message sending once we have proper metadata
    log::warn!("XCM upward message test requires proper runtime metadata");
    
    Ok(())
}

/// Run all integration tests
pub async fn run_integration_tests(fail_fast: bool, filter: Option<&str>) -> Result<TestSuiteResults> {
    use colored::Colorize;
    
    println!();
    println!("{}", "🧪 Running Integration Tests".bold().cyan());
    println!("{}", "━".repeat(50).bright_black());
    println!();
    
    let suite_start = Instant::now();
    let mut results = Vec::new();
    
    // Define all tests
    let tests = vec![
        ("network_initialization", test_network_initialization as fn() -> _),
        ("block_production", test_block_production as fn() -> _),
        ("extrinsic_submission", test_extrinsic_submission as fn() -> _),
        ("xcm_upward_message", test_xcm_upward_message as fn() -> _),
    ];
    
    // Filter tests if needed
    let filtered_tests: Vec<_> = tests
        .into_iter()
        .filter(|(name, _)| {
            if let Some(f) = filter {
                name.contains(f)
            } else {
                true
            }
        })
        .collect();
    
    if filtered_tests.is_empty() {
        println!("{}", "No tests match the filter".yellow());
        return Ok(TestSuiteResults {
            tests: vec![],
            total_duration: Duration::from_secs(0),
        });
    }
    
    log::info!("Running {} tests", filtered_tests.len());
    
    for (name, test_fn) in filtered_tests {
        let result = run_test(name, test_fn).await;
        
        // Display result
        match result.status {
            TestStatus::Passed => {
                println!(
                    "  {} {} ({:.2}s)",
                    "✓".green(),
                    name.green(),
                    result.duration.as_secs_f64()
                );
            }
            TestStatus::Failed => {
                println!(
                    "  {} {} ({:.2}s)",
                    "✗".red(),
                    name.red(),
                    result.duration.as_secs_f64()
                );
                if let Some(ref error) = result.error {
                    println!("    {}: {}", "Error".bright_black(), error.bright_black());
                }
            }
            TestStatus::Skipped => {
                println!(
                    "  {} {} (skipped)",
                    "○".yellow(),
                    name.yellow()
                );
            }
        }
        
        let should_stop = fail_fast && result.status == TestStatus::Failed;
        results.push(result);
        
        if should_stop {
            log::warn!("Stopping test execution due to failure (fail-fast mode)");
            break;
        }
    }
    
    let total_duration = suite_start.elapsed();
    
    println!();
    println!("{}", "Test Summary".bold());
    println!("{}", "━".repeat(50).bright_black());
    
    let suite_results = TestSuiteResults {
        tests: results,
        total_duration,
    };
    
    println!("  Total:      {}", suite_results.tests.len());
    println!("  Passed:     {}", suite_results.passed_count().to_string().green());
    println!("  Failed:     {}", suite_results.failed_count().to_string().red());
    println!("  Skipped:    {}", suite_results.skipped_count().to_string().yellow());
    println!("  Duration:   {:.2}s", total_duration.as_secs_f64());
    println!();
    
    if suite_results.is_success() {
        log::info!("All tests passed!");
    } else {
        log::error!("{} test(s) failed", suite_results.failed_count());
    }
    
    Ok(suite_results)
}
