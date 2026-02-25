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
//! Integration test modules and runner.

pub mod network;
pub mod xcm;
pub mod cps;
pub mod claim;

use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use crate::cli::NetworkTopology;

// Re-export test functions
use network::{test_network_initialization, test_block_production, test_extrinsic_submission};
use xcm::{test_xcm_upward_message, test_xcm_downward_message, test_xcm_token_teleport};
use cps::test_cps_pallet;
use claim::test_claim_pallet;

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

