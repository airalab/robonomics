///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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
//! Console line interface.

#![warn(unused_extern_crates)]

#[cfg(feature = "full")]
pub mod chain_spec;

#[macro_use]
#[cfg(feature = "full")]
pub mod service;

#[macro_use]
#[cfg(feature = "parachain")]
pub mod parachain;

#[cfg(feature = "sc-cli")]
mod cli;
#[cfg(feature = "sc-cli")]
mod command;

#[cfg(feature = "sc-cli")]
pub use cli::*;
#[cfg(feature = "sc-cli")]
pub use command::*;
#[cfg(feature = "sc-cli")]
pub use sc_cli::{Error, Result};
