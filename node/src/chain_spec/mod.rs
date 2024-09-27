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

use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use serde::{Deserialize, Serialize};
use sp_core::{Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

pub mod dev;
pub mod mainnet;
//pub mod testnet;

/// Robonomics runtime family chains.
pub enum RobonomicsFamily {
    /// Development chain (used for local tests only).
    Development,
    /// Ipci Network Parachain (https://ipci.io).
    ParachainIpci,
    /// Robonomics testnet parachain
    ParachainAlpha,
    /// Robonomics Kusama parachain
    ParachainKusama,
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

        if self.id() == "robonomics" {
            return RobonomicsFamily::ParachainKusama;
        }

        if self.id() == "ipci" {
            return RobonomicsFamily::ParachainIpci;
        }

        RobonomicsFamily::ParachainAlpha
    }
}

/// Generic extensions for Parachain ChainSpecs.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
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

/// General signer type.
pub type AccountPublic = <robonomics_primitives::Signature as Verify>::Signer;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> robonomics_primitives::AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}
