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

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    traits::{Get, UnfilteredDispatchable},
    weights::{GetDispatchInfo, Pays, Weight},
    Parameter,
};
use frame_system::{ensure_root, ensure_signed};
use sp_runtime::{
    traits::{SaturatedConversion, StaticLookup},
    DispatchResult, Perbill,
};
use sp_std::prelude::*;

/// RWS module main trait.
pub trait Config: pallet_timestamp::Config {
    /// Call subscription method.
    type Call: Parameter + UnfilteredDispatchable<Origin = Self::Origin> + GetDispatchInfo;
    /// The top limit weight for signle call.
    type WeightLimit: Get<Weight>;
    /// Transactions bandwidth allocated for subscription (in TPS).
    type TotalBandwidth: Get<u64>;
    /// Limit for quota points accumulation.
    type PointsLimit: Get<u64>;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
}

/// One call cost in quota points (points for 1 sec).
pub const CALL_COST: u64 = 1_000_000_000;

decl_error! {
    pub enum Error for Module<T: Config> {
        /// The origin account have no enough quota to process these call.
        NoQuota,
        /// The call does not meet the requirements.
        BadCall,
        /// This call is for oracle only.
        OracleOnlyCall,
    }
}

decl_event! {
    pub enum Event<T>
    where AccountId = <T as frame_system::Config>::AccountId,
    {
        /// RWS subscription registered.
        Subscription(AccountId, Perbill),
        /// Runtime method executed using RWS subscription.
        NewCall(AccountId, DispatchResult),
    }
}

decl_storage! {
    trait Store for Module<T: Config> as RWS {
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
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Authenticates the RWS staker and dispatches a free function call.
        ///
        /// The dispatch origin for this call must be _Signed_.
        ///
        /// # <weight>
        /// - Dependes of call method.
        /// - Basically this sould be free by concept.
        /// # </weight>
        #[weight = (0, call.get_dispatch_info().class, Pays::No)]
        fn call(origin, call: Box<<T as Config>::Call>) {
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

        /// Change account bandwidth share rate by authority.
        ///
        /// The dispatch origin for this call must be _oracle_.
        ///
        /// # <weight>
        /// - O(1).
        /// - Limited storage reads.
        /// - One DB change.
        /// # </weight>
        #[weight = 0]
        fn set_bandwidth(origin, source: <T::Lookup as StaticLookup>::Source, share: Perbill) {
            let sender = ensure_signed(origin)?;
            ensure!(sender == <Oracle<T>>::get(), Error::<T>::OracleOnlyCall);
            <Bandwidth<T>>::insert(T::Lookup::lookup(source)?, share);
        }
    }
}

impl<T: Config> Module<T> {
    /// Check staker quota for execute call.
    fn check_quota(staker: T::AccountId) -> bool {
        let share = <Bandwidth<T>>::get(staker.clone());
        if share == Default::default() {
            // Deny execution without minimum permissions.
            return false;
        }

        let now = pallet_timestamp::Module::<T>::get();
        let (last_active, points) = <Quota<T>>::get(staker.clone());
        if last_active == Default::default() {
            <Quota<T>>::insert(staker, (now, 0));
            // Quota points initialized, permit to call one time.
            return true;
        }

        let delta = now - last_active;
        let new_points = Self::estimate_points(share, delta.saturated_into::<u64>(), points);
        if new_points < CALL_COST {
            false
        } else {
            <Quota<T>>::insert(staker, (now, new_points - CALL_COST));
            true
        }
    }

    /// Check call to be executed via RWS.
    fn check_call(call: Box<<T as Config>::Call>) -> bool {
        // RWS calls weight should be lower than limit
        call.get_dispatch_info().weight < T::WeightLimit::get()
        // TODO: call internals filtering
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
    use pallet_robonomics_datalog as datalog;

    use frame_support::{
        assert_err, assert_ok, impl_outer_dispatch, impl_outer_origin, parameter_types,
        weights::Weight,
    };
    use node_primitives::Moment;
    use sp_core::H256;
    use sp_runtime::{testing::Header, traits::IdentityLookup, DispatchError, Perbill};

    impl_outer_origin! {
        pub enum Origin for Runtime {}
    }

    impl_outer_dispatch! {
        pub enum Call for Runtime where origin: Origin {
            rws::RWS,
            datalog::Datalog,
        }
    }

    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct Runtime;

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
    }

    impl frame_system::Config for Runtime {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Call = Call;
        type Hash = H256;
        type Hashing = sp_runtime::traits::BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = ();
        type BlockHashCount = BlockHashCount;
        type Version = ();
        type PalletInfo = ();
        type AccountData = ();
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type DbWeight = ();
        type BaseCallFilter = ();
        type SystemWeightInfo = ();
        type BlockWeights = ();
        type BlockLength = ();
    }

    parameter_types! {
        pub const MinimumPeriod: Moment = 5;
    }

    impl pallet_timestamp::Config for Runtime {
        type Moment = Moment;
        type OnTimestampSet = ();
        type MinimumPeriod = ();
        type WeightInfo = ();
    }

    impl datalog::Config for Runtime {
        type Record = bool;
        type Event = ();
        type Time = Timestamp;
    }

    parameter_types! {
        pub const WeightLimit: Weight = 1_000_000_000_000;
        pub const TotalBandwidth: u64 = 100;
        pub const PointsLimit: u64 = 1_000_000_000_000_000;
    }

    impl Config for Runtime {
        type TotalBandwidth = TotalBandwidth;
        type WeightLimit = WeightLimit;
        type PointsLimit = PointsLimit;
        type Event = ();
        type Call = Call;
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();
        storage.into()
    }

    type Timestamp = pallet_timestamp::Module<Runtime>;
    type Datalog = datalog::Module<Runtime>;
    type RWS = Module<Runtime>;

    #[test]
    fn test_set_oracle() {
        let oracle = 1;
        new_test_ext().execute_with(|| {
            assert_err!(
                RWS::set_oracle(Origin::none(), oracle),
                DispatchError::BadOrigin
            );

            assert_err!(
                RWS::set_oracle(Origin::signed(oracle), oracle),
                DispatchError::BadOrigin
            );

            assert_ok!(RWS::set_oracle(Origin::root(), oracle),);
            assert_eq!(RWS::oracle(), oracle);
        })
    }

    #[test]
    fn test_set_bandwidth() {
        let oracle = 1;
        let alice = 2;
        new_test_ext().execute_with(|| {
            assert_ok!(RWS::set_oracle(Origin::root(), oracle),);

            assert_err!(
                RWS::set_bandwidth(Origin::none(), alice, Default::default()),
                DispatchError::BadOrigin
            );

            assert_ok!(RWS::set_bandwidth(
                Origin::signed(oracle),
                alice,
                Perbill::from_percent(1)
            ),);
            assert_eq!(RWS::bandwidth(alice), Perbill::from_percent(1));
        })
    }

    #[test]
    fn test_transaction() {
        let oracle = 1;
        let alice = 2;

        new_test_ext().execute_with(|| {
            Timestamp::set_timestamp(1600438152000);

            assert_ok!(RWS::set_oracle(Origin::root(), oracle),);

            let call = Call::from(datalog::Call::record(true));

            assert_eq!(RWS::quota(oracle), (0, 0));
            assert_err!(
                RWS::call(Origin::signed(oracle), call.clone().into()),
                Error::<Runtime>::NoQuota,
            );

            assert_ok!(RWS::set_bandwidth(
                Origin::signed(oracle),
                alice,
                Perbill::from_percent(1)
            ),);
            assert_eq!(RWS::quota(alice), (0, 0));
            assert_ok!(RWS::call(Origin::signed(alice), call.clone().into()));
            assert_eq!(RWS::quota(alice), (1600438152000, 0));

            Timestamp::set_timestamp(1600438156000);

            assert_ok!(RWS::call(Origin::signed(alice), call.clone().into()));
            assert_eq!(RWS::quota(alice), (1600438156000, 3 * CALL_COST));

            assert_ok!(RWS::call(Origin::signed(alice), call.clone().into()));
            assert_eq!(RWS::quota(alice), (1600438156000, 2 * CALL_COST));

            assert_ok!(RWS::call(Origin::signed(alice), call.clone().into()));
            assert_eq!(RWS::quota(alice), (1600438156000, 1 * CALL_COST));

            assert_ok!(RWS::call(Origin::signed(alice), call.clone().into()));
            assert_eq!(RWS::quota(alice), (1600438156000, 0));

            assert_err!(
                RWS::call(Origin::signed(alice), call.into()),
                Error::<Runtime>::NoQuota,
            );
        })
    }
}
