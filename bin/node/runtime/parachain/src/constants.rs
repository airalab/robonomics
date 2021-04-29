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
//! A set of constant values used in substrate runtime.

/// Money matters.
pub mod currency {
    #[cfg(feature = "std")]
    use hex_literal::hex;
    #[cfg(feature = "std")]
    use node_primitives::AccountId;
    use node_primitives::Balance;

    pub const COASE: Balance = 1_000;
    pub const GLUSHKOV: Balance = 1_000 * COASE;
    pub const XRT: Balance = 1_000 * GLUSHKOV;

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 150 * GLUSHKOV / 100 + (bytes as Balance) * 60 * GLUSHKOV
    }

    #[cfg(feature = "std")]
    lazy_static::lazy_static! {
        pub static ref STAKE_HOLDERS: Vec<(AccountId, Balance)> = sp_std::vec![
            (AccountId::from(hex!["5c63763273b539fa6ed09b6b9844553922f7c5eb30195062b139b057ac861568"]), 1000 * XRT),
            (AccountId::from(hex!["caafae0aaa6333fcf4dc193146945fe8e4da74aa6c16d481eef0ca35b8279d73"]), 5000 * XRT),
            (AccountId::from(hex!["9c322cfa42b80ffb1fa0a096ffbbe08ff44423ea7e6626183ba14bfb20c98c53"]), 5305599999),
            (AccountId::from(hex!["1a84dfd9e4e30b0d48c4110bf7c509d5f27a68d4fade696dff3274e0afa09062"]), 1 * XRT),
            (AccountId::from(hex!["8e5cda83432e069937b7e032ed8f88280a020aba933ee928eb936ab265f4c364"]), 10_000 * XRT),
        ];
    }
}

/// Time.
pub mod time {
    use node_primitives::{BlockNumber, Moment};
    pub const MILLISECS_PER_BLOCK: Moment = 12000;
    pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;
    pub const MINUTES: BlockNumber = 60 / (SECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;
}

// CRITICAL NOTE: The system module maintains two constants: a _maximum_ block weight and a
// _ratio_ of it yielding the portion which is accessible to normal transactions (reserving the rest
// for operational ones). `TARGET_BLOCK_FULLNESS` is entirely independent and the system module is
// not aware of if, nor should it care about it. This constant simply denotes on which ratio of the
// _maximum_ block weight we tweak the fees. It does NOT care about the type of the dispatch.
//
// For the system to be configured in a sane way, `TARGET_BLOCK_FULLNESS` should always be less than
// the ratio that `system` module uses to find normal transaction quota.
/// Fee-related.
pub mod fee {
    pub use sp_runtime::Perbill;

    /// The block saturation level. Fees will be updates based on this value.
    pub const TARGET_BLOCK_FULLNESS: Perbill = Perbill::from_percent(25);
}
