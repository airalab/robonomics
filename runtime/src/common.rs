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
//! A set of common values used in robonomics runtime.

pub use sp_runtime::{generic, traits::{IdentifyAccount, Verify}};
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = sp_runtime::MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them.
pub type AccountIndex = u32;

/// Id used for identifying assets.
pub type AssetId = u32;

/// Balance of an account.
pub type Balance = u128;

/// The amount type, should be signed version of balance.
pub type Amount = i128;

/// Type used for expressing timestamp.
pub type Moment = u64;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// A timestamp: milliseconds since the unix epoch.
/// `u64` is enough to represent a duration of half a billion years, when the
/// time scale is milliseconds.
pub type Timestamp = u64;

/// Money matters.
pub mod currency {
    use super::*;
    use hex_literal::hex;

    pub const COASE: Balance = 1_000;
    pub const GLUSHKOV: Balance = 1_000 * COASE;
    pub const XRT: Balance = 1_000 * GLUSHKOV;

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 150 * GLUSHKOV / 100 + (bytes as Balance) * 60 * GLUSHKOV
    }

    /// ERC20 Robonomics Token: https://etherscan.io/token/0x7de91b204c1c737bcee6f000aaa6569cf7061cb7
    pub const ERC20_XRT_ADDRESS: [u8; 20] = hex!["7de91b204c1c737bcee6f000aaa6569cf7061cb7"];

    /// Set of community accounts.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum CommunityAccount {
        /// Treasury manage community funds via open governance.
        Treasury,
    }

    impl IdentifyAccount for CommunityAccount {
        type AccountId = AccountId;
        fn into_account(self) -> Self::AccountId {
            match self {
                CommunityAccount::Treasury => AccountId::from(hex![
                    "6d6f646c70792f74727372790000000000000000000000000000000000000000"
                ]),
            }
        }
    }
}

/// Time constants.
pub mod time {
    use super::{BlockNumber, Moment};

    pub const MILLISECS_PER_BLOCK: Moment = 6000;
    pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;
    pub const MINUTES: BlockNumber = 60 / (SECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;
}

