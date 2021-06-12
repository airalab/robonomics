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
    use robonomics_primitives::AccountId;
    use robonomics_primitives::Balance;

    pub const COASE: Balance = 1_000;
    pub const GLUSHKOV: Balance = 1_000 * COASE;
    pub const XRT: Balance = 1_000 * GLUSHKOV;

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 150 * GLUSHKOV / 100 + (bytes as Balance) * 60 * GLUSHKOV
    }

    #[cfg(feature = "std")]
    lazy_static::lazy_static! {
        pub static ref STAKE_HOLDERS: Vec<(AccountId, Balance)> = sp_std::vec![
            // snapshot of 12.06.2021
            (AccountId::from(hex!["5c63763273b539fa6ed09b6b9844553922f7c5eb30195062b139b057ac861568"]), 1_000 * XRT),
            (AccountId::from(hex!["caafae0aaa6333fcf4dc193146945fe8e4da74aa6c16d481eef0ca35b8279d73"]), 5_000 * XRT),
            (AccountId::from(hex!["1a84dfd9e4e30b0d48c4110bf7c509d5f27a68d4fade696dff3274e0afa09062"]), 1 * XRT),
            (AccountId::from(hex!["8e5cda83432e069937b7e032ed8f88280a020aba933ee928eb936ab265f4c364"]), 10_000 * XRT),
            (AccountId::from(hex!["4468ebb7dbdc05fa9c062583fffe942179dcd8b0dff095b265f35fb3c936305d"]), 10 * XRT),
            (AccountId::from(hex!["c8bdd564dbb9de6c2ded0f1c7649195336ea9a11246499257ecc5d72bc544d24"]), 150 * GLUSHKOV),
            (AccountId::from(hex!["d66ac04ad10dbb5ac16a6a477dce727d647cbfd809d18b150cb402ae1569b555"]), 54983128556),
            (AccountId::from(hex!["2e701510ea99739603eca9b602090f9c9292d3f17ba7a2a58d1a99c26c122372"]), 25745228238),
            (AccountId::from(hex!["7c419853e4527f91c10475108c08c41488ea4c90394a18b48bf5a6dc0c144079"]), 13541598278),
            (AccountId::from(hex!["140bd550d9367068fe97a6065d50bc8bdf41512c3c056532f314efaa9b407243"]), 214114327643),
            (AccountId::from(hex!["20a1927f851b85a2666b6974291e1b3d080aa4fdf16b7f7a96e12767c8323116"]), 1483287492836),
            (AccountId::from(hex!["0c48e3ac08805cb181484c6bdae2b3c1800cff183fe6662657068883fe923363"]), 55670405588),
            (AccountId::from(hex!["04f2c20a93384e68602305e4ee2fab21efd32b38fe821004339d695096726731"]), 205356000000),
            (AccountId::from(hex!["140bd550d9367068fe97a6065d50bc8bdf41512c3c056532f314efaa9b407243"]), 51499079036),
            (AccountId::from(hex!["10207cbf5dca98b9915afe6ae001df7d81cd8fd2090a2f150e17ab7751894537"]), 2638200000),
            (AccountId::from(hex!["ae59474d9bad862954207f1fafc8be18b066e9c20ef776077cf742e6ccb13548"]), 2_000 * XRT),
            (AccountId::from(hex!["0c8220c20b57d955cd84344bcb97955704f70c88c037a2811b92ba8b81ceed18"]), 1_620 * XRT),
            (AccountId::from(hex!["1e65015f1fcf3b5f0dad80efb6213321b2fbace6b1662722d574d9641bbfa072"]), 10083703970),
            (AccountId::from(hex!["1c12b0d4a58e59124e863a171252a47939e4b7f3131534f9477c0020e04dfd1e"]), 4922233240),
            (AccountId::from(hex!["ba19513146f4ac49ef7ffc6eca82d823d741a487bd0b3d9c5b17ae04a4191c48"]), 3_000 * XRT),
            (AccountId::from(hex!["ecc01bcc8ad8bac7406622a18a977ac862ac57a65444f7ec426d9ad20c65c056"]), 331285522627),
            (AccountId::from(hex!["441af87350235ec135c2e388807249f22460588e3f68ea8f6e6cfd7af9159f43"]), 10132996319),
            (AccountId::from(hex!["68ebabf73c36d3f48c9b9c63f681686a94a1b7208b821c34db4d64e1be85e616"]), 18103206092),
            // DAO (https://etherscan.io/tx/0x6b9a9cbe7d21badf565ebce0fb50b865da8f5f784899db5fb455d1b276d14acf)
            (AccountId::from(hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"]), 692500 * XRT),
        ];
    }
}

/// Time.
pub mod time {
    use robonomics_primitives::{BlockNumber, Moment};
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
