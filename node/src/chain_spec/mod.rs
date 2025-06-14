///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2024 Robonomics Network <research@robonomics.network>
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
//! Chain specification and utils.

use sc_chain_spec::ChainSpecExtension;
use serde::{Deserialize, Serialize};

pub mod dev;
pub mod mainnet;

/// Robonomics runtime family chains.
pub enum RobonomicsFamily {
    /// Development chain (used for local tests only).
    Development,
    /// Robonomics mainnet parachain
    Mainnet,
}

/// Robonomics family chains idetify.
pub trait RobonomicsChain {
    fn family(&self) -> RobonomicsFamily;
}

impl RobonomicsChain for Box<dyn sc_chain_spec::ChainSpec> {
    fn family(&self) -> RobonomicsFamily {
        if self.id() == "dev" {
            return RobonomicsFamily::Development;
        }

        return RobonomicsFamily::Mainnet;
    }
}

/// Generic extensions for Parachain ChainSpecs.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
        sc_chain_spec::get_extension(chain_spec.extensions())
    }
}

/// Chain Specification.
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;
