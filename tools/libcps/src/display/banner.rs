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
//! ASCII art banners and animations for terminal display.

use colored::*;
use std::thread;
use std::time::Duration;

/// Print the main LIBCPS banner
pub fn print_banner() {
    println!(
        "{}",
        r#"
    ╔═══════════════════════════════════════════════════════════════╗
    ║                                                               ║
    ║   ██╗     ██╗██████╗  ██████╗██████╗ ███████╗                ║
    ║   ██║     ██║██╔══██╗██╔════╝██╔══██╗██╔════╝                ║
    ║   ██║     ██║██████╔╝██║     ██████╔╝███████╗                ║
    ║   ██║     ██║██╔══██╗██║     ██╔═══╝ ╚════██║                ║
    ║   ███████╗██║██████╔╝╚██████╗██║     ███████║                ║
    ║   ╚══════╝╚═╝╚═════╝  ╚═════╝╚═╝     ╚══════╝                ║
    ║                                                               ║
    ║           Cyber-Physical Systems - Robonomics Network         ║
    ║                                                               ║
    ╚═══════════════════════════════════════════════════════════════╝
"#
        .bright_cyan()
        .bold()
    );
}

/// Print a section header with ASCII art
pub fn print_section(title: &str) {
    println!("\n{}", format!("  ┌─[ {} ]", title).bright_yellow().bold());
    println!("{}", "  │".bright_yellow());
}

/// Close a section
pub fn close_section() {
    println!("{}", "  └─".bright_yellow());
}

/// Animated spinner for operations
pub fn spinner(msg: &str, duration_ms: u64) {
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let iterations = duration_ms / 100;

    for i in 0..iterations {
        let frame = frames[i as usize % frames.len()];
        print!("\r{} {} ", frame.cyan().bold(), msg.cyan());
        std::io::Write::flush(&mut std::io::stdout()).ok();
        thread::sleep(Duration::from_millis(100));
    }
    println!();
}

/// ASCII progress bar
pub fn progress_bar(current: usize, total: usize, width: usize) {
    let percent = (current as f64 / total as f64 * 100.0) as usize;
    let filled = (current as f64 / total as f64 * width as f64) as usize;
    let empty = width - filled;

    print!(
        "\r[{}{}] {}%",
        "=".repeat(filled).green(),
        " ".repeat(empty),
        percent
    );
    std::io::Write::flush(&mut std::io::stdout()).ok();

    if current == total {
        println!();
    }
}

/// Box drawing for important messages
pub fn message_box(title: &str, lines: &[&str]) {
    let max_len = lines
        .iter()
        .map(|l| l.len())
        .max()
        .unwrap_or(0)
        .max(title.len());
    let width = max_len + 4;

    // Top border
    println!("\n  ┌─{}─┐", "─".repeat(width));

    // Title
    let padding = width - title.len() - 2;
    let left_pad = padding / 2;
    let right_pad = padding - left_pad;
    println!(
        "  │ {}{}{} │",
        " ".repeat(left_pad),
        title.bright_white().bold(),
        " ".repeat(right_pad)
    );

    // Separator
    println!("  ├─{}─┤", "─".repeat(width));

    // Lines
    for line in lines {
        let line_padding = width - line.len() - 2;
        println!("  │  {}{} │", line, " ".repeat(line_padding));
    }

    // Bottom border
    println!("  └─{}─┘\n", "─".repeat(width));
}

/// Animated typing effect for text
pub fn typewriter(text: &str, delay_ms: u64) {
    for ch in text.chars() {
        print!("{}", ch);
        std::io::Write::flush(&mut std::io::stdout()).ok();
        thread::sleep(Duration::from_millis(delay_ms));
    }
    println!();
}

/// ASCII art separator
pub fn separator() {
    println!(
        "{}",
        "    ════════════════════════════════════════════════════════".bright_black()
    );
}

/// Print a boxed message with ASCII art
pub fn boxed_message(msg: &str) {
    let len = msg.len();
    println!("\n  ┌─{}─┐", "─".repeat(len + 2));
    println!("  │ {} │", msg.bright_white());
    println!("  └─{}─┘", "─".repeat(len + 2));
}
