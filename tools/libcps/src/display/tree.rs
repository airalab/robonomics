///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2025 Robonomics Network <research@robonomics.network>
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
//! Tree visualization utilities.

use colored::*;
use sp_core::crypto::{AccountId32, Ss58Codec};

/// Print a beautiful tree structure for a single CPS node (non-recursive: shows this node's
/// details and lists child IDs, but does not recursively display full child node details)
pub fn print_tree(
    node_id: u64,
    owner: AccountId32,
    meta: Option<&str>,
    payload: Option<&str>,
    children: &[u64],
) {
    println!(
        "\n{} {}\n",
        "[*] CPS Node".bright_cyan().bold(),
        format!("ID: {node_id}").bright_white().bold()
    );

    // Owner
    println!(
        "{}  {} {}",
        "|--".bright_black(),
        "[O] Owner:".bright_yellow(),
        owner.to_ss58check().bright_white()
    );

    // Metadata
    if let Some(meta_str) = meta {
        println!(
            "{}  {} {}",
            "|--".bright_black(),
            "[M] Meta:".bright_magenta(),
            format_data(meta_str)
        );
    } else {
        println!(
            "{}  {} {}",
            "|--".bright_black(),
            "[M] Meta:".bright_magenta(),
            "<empty>".bright_black()
        );
    }

    // Payload
    if let Some(payload_str) = payload {
        println!(
            "{}  {} {}",
            "`--".bright_black(),
            "[P] Payload:".bright_green(),
            format_data(payload_str)
        );
    } else {
        println!(
            "{}  {} {}",
            "`--".bright_black(),
            "[P] Payload:".bright_green(),
            "<empty>".bright_black()
        );
    }

    // Children
    if !children.is_empty() {
        println!(
            "\n{}  {} {}",
            "   ".bright_black(),
            "[C] Children:".bright_blue(),
            format!("({} nodes)", children.len()).bright_black()
        );
        for (i, child_id) in children.iter().enumerate() {
            let prefix = if i == children.len() - 1 {
                "`--"
            } else {
                "|--"
            };
            println!(
                "      {} {} {}",
                prefix.bright_black(),
                "Node:".bright_white(),
                child_id.to_string().bright_cyan()
            );
        }
    } else {
        println!(
            "\n{}  {} {}",
            "   ".bright_black(),
            "[C] Children:".bright_blue(),
            "<none>".bright_black()
        );
    }

    println!();
}

fn format_data(data: &str) -> ColoredString {
    // Try to parse as JSON for pretty formatting
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
        serde_json::to_string_pretty(&json)
            .unwrap_or_else(|_| data.to_string())
            .bright_white()
    } else if data.starts_with("{") && data.contains("\"version\"") {
        // Encrypted data
        "[Encrypted]".bright_red().bold()
    } else if data.len() > 100 {
        // Long data, truncate
        format!("{}...", &data[..97]).bright_white()
    } else {
        data.bright_white()
    }
}

/// Print a node in recursive tree format (for building full tree visualizations)
pub fn print_node_recursive(
    node_id: u64,
    owner: AccountId32,
    meta: Option<&str>,
    payload: Option<&str>,
    children: &[u64],
    prefix: &str,
    is_last: bool,
) {
    // Node header with proper tree symbols
    let branch = if is_last { "`--" } else { "|--" };

    println!(
        "{}{} {} {}",
        prefix,
        branch.bright_black(),
        "[*]".bright_cyan().bold(),
        format!("Node {}", node_id).bright_white().bold()
    );

    // Calculate the prefix for this node's content
    let content_prefix = if is_last {
        format!("{}    ", prefix)
    } else {
        format!("{}|   ", prefix)
    };

    // Owner
    println!(
        "{}{}  {} {}",
        content_prefix,
        "|--".bright_black(),
        "[O]".bright_yellow(),
        owner.to_ss58check().bright_white()
    );

    // Metadata
    if let Some(meta_str) = meta {
        println!(
            "{}{}  {} {}",
            content_prefix,
            "|--".bright_black(),
            "[M]".bright_magenta(),
            format_data(meta_str)
        );
    } else {
        println!(
            "{}{}  {} {}",
            content_prefix,
            "|--".bright_black(),
            "[M]".bright_magenta(),
            "<empty>".bright_black()
        );
    }

    // Payload
    if let Some(payload_str) = payload {
        println!(
            "{}{}  {} {}",
            content_prefix,
            "`--".bright_black(),
            "[P]".bright_green(),
            format_data(payload_str)
        );
    } else {
        println!(
            "{}{}  {} {}",
            content_prefix,
            "`--".bright_black(),
            "[P]".bright_green(),
            "<empty>".bright_black()
        );
    }

    // Note: Children are printed by the recursive caller, not here
}

pub fn success(msg: &str) {
    println!("{} {}", "[+]".green().bold(), msg.green());
}

/*
pub fn error(msg: &str) {
    eprintln!("{} {}", "[!]".red().bold(), msg.red());
}
*/

pub fn info(msg: &str) {
    println!("{} {}", "[i]".blue().bold(), msg.bright_blue());
}

pub fn warning(msg: &str) {
    println!("{} {}", "[!]".yellow().bold(), msg.yellow());
}

pub fn progress(msg: &str) {
    println!("{} {}", "[~]".cyan().bold(), msg.cyan());
}
