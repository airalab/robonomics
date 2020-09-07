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
//! Robonomics Web Services runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, EncodeLike, Encode, Decode};
use sp_runtime::{Perbill, DispatchResult, traits::StaticLookup};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, Parameter,
    traits::{UnfilteredDispatchable, Get},
    weights::{GetDispatchInfo, Weight},
};
use frame_system::{ensure_signed, ensure_root};
use sp_std::{convert::TryInto, prelude::*};

/// RWS module main trait.
pub trait Trait: pallet_timestamp::Trait {
    /// Call subscription method.
    type Call: Parameter + UnfilteredDispatchable<Origin=Self::Origin> + GetDispatchInfo;
    /// The top limit weight for signle call.
    type WeightLimit: Get<Weight>;
    /// Transactions bandwidth allocated for subscription (in TPS).
    type TotalBandwidth: Get<u64>;
    /// Limit for quota points accumulation.
    type PointsLimit: Get<u64>;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

/// One call cost in quota points (points for 1 sec).
const CALL_COST: u64 = 1_000_000_000;

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// The origin account have no enough quota to process these call.
        NoQuota,
        /// The call does not meet the requirements.
        BadCall,
    }
}

decl_event! {
    pub enum Event<T>
    where AccountId = <T as frame_system::Trait>::AccountId,
    {
        /// RWS subscription registered.
        Subscription(AccountId, Perbill),
        /// Runtime method executed using RWS subscription.
        NewCall(AccountId, DispatchResult),
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as RWS {
        /// The `AccountId` of Ethereum oracle.
        Oracle get(fn oracle) config(): T::AccountId;
        /// Bandwidth allocation for account.
        Bandwidth get(fn bandwidth) config():
            map hasher(twox_64_concat) T::AccountId => Perbill;
        /// Quota acconting, transaction quota grown while account idle.
        Quota get(fn quota):
            map hasher(twox_64_concat) T::AccountId => (T::Moment, u64);
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Authenticates the RWS staker and dispatches a free function call.
        ///
        /// The dispatch origin for this call must be _Signed_.
        ///
        /// # <weight>
        /// - Dependes of call method.
        /// - Basically this sould be free by concept.
        /// # </weight>
        #[weight = (0, call.get_dispatch_info().class)]
        fn call(origin, call: Box<<T as Trait>::Call>) {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;
            ensure!(Self::check_call(call.clone()), Error::<T>::BadCall);
            ensure!(Self::check_quota(sender.clone()), Error::<T>::NoQuota);

            let res = call.dispatch_bypass_filter(frame_system::RawOrigin::Signed(sender.clone()).into());
            Self::deposit_event(RawEvent::NewCall(sender, res.map(|_| ()).map_err(|e| e.error)));
        }

        /// Change RWS oracle account. 
        ///
        /// The dispatch origin for this call must be _Root_.
        ///
        /// # <weight>
        /// - O(1).
        /// - Limited storage reads.
        /// - One DB change.
        /// # </weight>
        #[weight = 0]
        fn set_oracle(origin, new: <T::Lookup as StaticLookup>::Source) {
            ensure_root(origin)?;
            <Oracle<T>>::put(T::Lookup::lookup(new)?);
        }
    }
}

impl<T: Trait> Module<T> {
    /// Check staker quota for execute call.
    fn check_quota(staker: T::AccountId) -> bool {
        let now = pallet_timestamp::Module::<T>::get();

        let (last_active, points) = <Quota<T>>::get(staker.clone());
        if last_active == Default::default() {
            <Quota<T>>::insert(staker, (now, 0));
            // Quota points initialized, permit to call one time.
            return true
        }

        let delta = (now - last_active).try_into().unwrap_or(0) as u64;
        let share = <Bandwidth<T>>::get(staker.clone());
        let new_points = Self::estimate_points(share, delta, points);
        if new_points < CALL_COST {
            false
        } else {
            <Quota<T>>::insert(staker, (now, new_points - CALL_COST));
            true
        }
    }

    /// Check call to be executed via RWS.
    fn check_call(call: Box<<T as Trait>::Call>) -> bool {
        // RWS calls weight should be lower than limit
        call.get_dispatch_info().weight < T::WeightLimit::get()
    }

    /// Estimate quota points for given share rate, timedelta and previous value.
    fn estimate_points(share: Perbill, delta: u64, points: u64) -> u64 {
        // Simple filter to prevent excessive point accumulation.
        if points > T::PointsLimit::get() {
            points
        } else {
            points + share.mul_ceil(Self::total_points_ms() * delta)
        }
    }

    /// Total quota points in ms
    fn total_points_ms() -> u64 {
        // 1_000_000_000 points per sec 
        T::TotalBandwidth::get() * 1_000_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use frame_support::{
        assert_err, assert_ok, impl_outer_origin, parameter_types, weights::Weight,
    };
    use node_primitives::Moment;
    use sp_core::H256;
    use sp_runtime::{testing::Header, traits::IdentityLookup, DispatchError, Perbill};

    impl_outer_origin! {
        pub enum Origin for Runtime {}
    }

    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct Runtime;

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub const MaximumBlockWeight: Weight = 1024;
        pub const MaximumBlockLength: u32 = 2 * 1024;
        pub const AvailableBlockRatio: Perbill = Perbill::one();
    }

    impl frame_system::Trait for Runtime {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Call = ();
        type Hash = H256;
        type Hashing = sp_runtime::traits::BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = ();
        type BlockHashCount = BlockHashCount;
        type MaximumBlockWeight = MaximumBlockWeight;
        type MaximumBlockLength = MaximumBlockLength;
        type AvailableBlockRatio = AvailableBlockRatio;
        type Version = ();
        type ModuleToIndex = ();
        type AccountData = ();
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type DbWeight = ();
        type BlockExecutionWeight = ();
        type ExtrinsicBaseWeight = ();
        type MaximumExtrinsicWeight = ();
        type BaseCallFilter = ();
        type SystemWeightInfo = ();
    }

    parameter_types! {
        pub const MinimumPeriod: Moment = 5;
    }

    impl pallet_timestamp::Trait for Runtime {
        type Moment = Moment;
        type OnTimestampSet = ();
        type MinimumPeriod = ();
        type WeightInfo = ();
    }

    parameter_types! {
        pub const WeightLimit: Weight = 1_000_000_000_000;
        pub const TotalBandwidth: u64 = 100;
        pub const PointsLimit: u64 1_000_000_000_000_000; 
    }

    impl Trait for Runtime {
        type Call = Call;
        type WeightLimit = WeightLimit;
        type TotalBandwidth = TotalBandwidth;
        type PointsLimit = PointsLimit;
        type Event = ();
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();
        storage.into()
    }

    type RWS = Module<Runtime>;

    #[test]
    fn test_oracle() {
        new_test_ext().execute_with(|| {
        })
    }
}
