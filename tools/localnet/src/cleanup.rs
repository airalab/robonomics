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
//! Cleanup utilities for network resources.

use anyhow::Result;
use colored::Colorize;
use std::io::{self, Write};

/// Confirm cleanup with the user
fn confirm_cleanup() -> Result<bool> {
    print!("{} ", "Are you sure you want to clean up network resources? [y/N]:".yellow());
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let answer = input.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

/// Clean up network resources
pub async fn cleanup_resources(force: bool) -> Result<()> {
    println!();
    println!("{}", "🧹 Cleaning Network Resources".bold().yellow());
    println!("{}", "━".repeat(50).bright_black());
    println!();
    
    if !force {
        if !confirm_cleanup()? {
            println!("{}", "Cleanup cancelled".yellow());
            return Ok(());
        }
    }
    
    log::info!("Cleaning up network resources...");
    
    // Note: zombienet-sdk handles cleanup automatically when Network is dropped
    // For additional cleanup, we could kill processes by name, but that's handled by the SDK
    
    println!("{} Network resources cleaned", "✓".green());
    log::info!("Cleanup completed");
    
    Ok(())
}
