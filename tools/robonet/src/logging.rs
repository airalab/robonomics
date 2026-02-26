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
//! Logging configuration and initialization.

use env_logger::Builder;
use log::LevelFilter;
use std::io::Write;

/// Initialize logging based on verbosity level
pub fn init_logger(verbose: u8) {
    let mut builder = Builder::new();

    // Set log level based on verbosity
    let level = match verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    builder
        .filter_level(level)
        .format(|buf, record| {
            use colored::Colorize;
            let level_colored = match record.level() {
                log::Level::Error => "ERROR".red().bold(),
                log::Level::Warn => "WARN".yellow().bold(),
                log::Level::Info => "INFO".green(),
                log::Level::Debug => "DEBUG".blue(),
                log::Level::Trace => "TRACE".purple(),
            };

            writeln!(buf, "[{}] {}", level_colored, record.args())
        })
        .init();
}

/// Initialize JSON logging for CI environments
pub fn init_json_logger() {
    let mut builder = Builder::new();

    builder
        .filter_level(LevelFilter::Info)
        .format(|buf, record| {
            let json = serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "level": record.level().to_string(),
                "message": record.args().to_string(),
                "target": record.target(),
            });
            writeln!(buf, "{}", json)
        })
        .init();
}
