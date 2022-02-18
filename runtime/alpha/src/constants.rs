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
    use robonomics_primitives::Balance;

    pub const COASE: Balance = 1_000;
    pub const GLUSHKOV: Balance = 1_000 * COASE;
    pub const XRT: Balance = 1_000 * GLUSHKOV;

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 150 * GLUSHKOV / 100 + (bytes as Balance) * 60 * GLUSHKOV
    }

    use frame_support::traits::{
	    fungibles::{self, Balanced, CreditOf},
	    Contains, Get,
    };
    use pallet_asset_tx_payment::HandleCredit;
    use sp_runtime::traits::Zero;
    use sp_std::marker::PhantomData;
    use xcm::latest::{AssetId, Fungibility::Fungible, MultiAsset, MultiLocation};
    use xcm_executor::traits::FilterAssetLocation;

    /// Type alias to conveniently refer to `frame_system`'s `Config::AccountId`.
    pub type AccountIdOf<R> = <R as frame_system::Config>::AccountId;

    /// A `HandleCredit` implementation that naively transfers the fees to the block author.
    /// Will drop and burn the assets in case the transfer fails.
    pub struct AssetsToBlockAuthor<R>(PhantomData<R>);
    impl<R> HandleCredit<AccountIdOf<R>, pallet_assets::Pallet<R>> for AssetsToBlockAuthor<R>
    where
        R: pallet_robonomics_lighthouse::Config + pallet_assets::Config,
        AccountIdOf<R>:
            From<polkadot_primitives::v1::AccountId> + Into<polkadot_primitives::v1::AccountId>,
    {
        fn handle_credit(credit: CreditOf<AccountIdOf<R>, pallet_assets::Pallet<R>>) {
            if let Some(author) = pallet_robonomics_lighthouse::Pallet::<R>::lighthouse() {
                // In case of error: Will drop the result triggering the `OnDrop` of the imbalance.
                let _ = pallet_assets::Pallet::<R>::resolve(&author, credit);
            }
        }
    }

    /// Allow checking in assets that have issuance > 0.
    pub struct NonZeroIssuance<AccountId, Assets>(PhantomData<(AccountId, Assets)>);
    impl<AccountId, Assets> Contains<<Assets as fungibles::Inspect<AccountId>>::AssetId>
        for NonZeroIssuance<AccountId, Assets>
    where
        Assets: fungibles::Inspect<AccountId>,
    {
        fn contains(id: &<Assets as fungibles::Inspect<AccountId>>::AssetId) -> bool {
            !Assets::total_issuance(*id).is_zero()
        }
    }

    /// Asset filter that allows all assets from a certain location.
    pub struct AssetsFrom<T>(PhantomData<T>);
    impl<T: Get<MultiLocation>> FilterAssetLocation for AssetsFrom<T> {
        fn filter_asset_location(asset: &MultiAsset, origin: &MultiLocation) -> bool {
            let loc = T::get();
            &loc == origin &&
                matches!(asset, MultiAsset { id: AssetId::Concrete(asset_loc), fun: Fungible(_a) }
                    if asset_loc.match_and_split(&loc).is_some())
        }
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
