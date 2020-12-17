///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life>
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

pub mod chain_spec;

#[macro_use]
mod service;

#[macro_use]
#[cfg(feature = "parachain")]
mod parachain;

#[cfg(feature = "browser")]
mod browser;
#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "cli")]
mod command;

#[cfg(feature = "browser")]
pub use browser::*;
#[cfg(feature = "cli")]
pub use cli::*;
#[cfg(feature = "cli")]
pub use command::*;
