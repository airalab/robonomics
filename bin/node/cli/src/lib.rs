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

pub mod chain_spec;

#[macro_use]
mod service;
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

/// Can be called for a `Configuration` to check if it is a configuration for IPCI network.
pub trait IsIpci {
    fn is_ipci(&self) -> bool;
}

/// The chain specification option.
#[derive(Clone, Debug, PartialEq)]
pub enum ChainSpec {
	/// Whatever the current runtime is, with just Alice as an auth.
	Development,
	/// Whatever the current runtime is, with simple Alice/Bob auths.
	LocalTestnet,
    /// Robonomics public testnet.
    RobonomicsTestnet,
    /// IPCI blockchain network.
    Ipci,
}

impl ChainSpec {
    /// Get an actual chain config from one of the alternatives.
    pub(crate) fn load(self) -> chain_spec::ChainSpec {
        match self {
            ChainSpec::Development       => chain_spec::development_testnet_config(),
            ChainSpec::LocalTestnet      => chain_spec::local_testnet_config(),
            ChainSpec::RobonomicsTestnet => chain_spec::robonomics_testnet_config(),
            ChainSpec::Ipci              => chain_spec::ipci_config(),
        }
    }

    pub(crate) fn from(s: &str) -> Option<Self> {
        match s {
            "dev"             => Some(ChainSpec::Development),
            "local"           => Some(ChainSpec::LocalTestnet),
            "ipci"            => Some(ChainSpec::Ipci),
            "" | "robonomics" => Some(ChainSpec::RobonomicsTestnet),
            _ => None,
        }
    }
}

pub fn load_spec(id: &str) -> Result<Option<chain_spec::ChainSpec>, String> {
    Ok(match ChainSpec::from(id) {
        Some(spec) => Some(spec.load()),
        None => None,
    })
}
