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
//! CI output formatting for machine-readable results.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::tests::{TestResult, TestStatus, TestSuiteResults};

#[derive(Debug, Serialize, Deserialize)]
pub struct CiTestResult {
    pub name: String,
    pub status: String,
    pub duration_secs: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CiOutput {
    pub status: String,
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_secs: f64,
    pub tests: Vec<CiTestResult>,
}

impl From<&TestResult> for CiTestResult {
    fn from(result: &TestResult) -> Self {
        CiTestResult {
            name: result.name.clone(),
            status: match result.status {
                TestStatus::Passed => "passed".to_string(),
                TestStatus::Failed => "failed".to_string(),
                TestStatus::Skipped => "skipped".to_string(),
            },
            duration_secs: result.duration.as_secs_f64(),
            error: result.error.clone(),
        }
    }
}

impl From<&TestSuiteResults> for CiOutput {
    fn from(results: &TestSuiteResults) -> Self {
        CiOutput {
            status: if results.is_success() {
                "success".to_string()
            } else {
                "failure".to_string()
            },
            total_tests: results.tests.len(),
            passed: results.passed_count(),
            failed: results.failed_count(),
            skipped: results.skipped_count(),
            duration_secs: results.total_duration.as_secs_f64(),
            tests: results.tests.iter().map(CiTestResult::from).collect(),
        }
    }
}

/// Output test results in JSON format for CI
pub fn output_json(results: &TestSuiteResults) -> Result<()> {
    let ci_output = CiOutput::from(results);
    let json = serde_json::to_string_pretty(&ci_output)?;
    println!("{}", json);
    Ok(())
}

/// Generate GitHub Actions annotations for errors
pub fn output_github_annotations(results: &TestSuiteResults) {
    for test in &results.tests {
        if test.status == TestStatus::Failed {
            if let Some(ref error) = test.error {
                println!("::error title=Test Failed: {}::{}", test.name, error);
            } else {
                println!("::error title=Test Failed: {}::Unknown error", test.name);
            }
        }
    }
}
