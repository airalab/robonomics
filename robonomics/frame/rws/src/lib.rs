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

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Time, UnfilteredDispatchable},
        weights::GetDispatchInfo,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{
        traits::{SaturatedConversion, StaticLookup},
        DispatchResult, Perbill,
    };
    use sp_std::prelude::*;

    /// One call cost in quota points (points for 1 sec).
    pub const CALL_COST: u64 = 1_000_000_000;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Call subscription method.
        type Call: Parameter + UnfilteredDispatchable<Origin = Self::Origin> + GetDispatchInfo;
        /// Current time source.
        type Time: Time;
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// The top limit weight for signle call.
        #[pallet::constant]
        type WeightLimit: Get<Weight>;
        /// Transactions bandwidth allocated for subscription (in TPS).
        #[pallet::constant]
        type TotalBandwidth: Get<u64>;
        /// Limit for quota points accumulation.
        #[pallet::constant]
        type PointsLimit: Get<u64>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// The origin account have no enough quota to process these call.
        NoQuota,
        /// The call does not meet the requirements.
        BadCall,
        /// This call is for oracle only.
        OracleOnlyCall,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        /// RWS subscription registered.
        Subscription(T::AccountId, Perbill),
        /// Runtime method executed using RWS subscription.
        NewCall(T::AccountId, DispatchResult),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::storage]
    #[pallet::getter(fn oracle)]
    /// The `AccountId` of Ethereum oracle.
    pub(super) type Oracle<T> = StorageValue<_, <T as frame_system::Config>::AccountId>;

    #[pallet::storage]
    #[pallet::getter(fn bandwidth)]
    /// Bandwidth allocation for account.
    pub(super) type Bandwidth<T> =
        StorageMap<_, Twox64Concat, <T as frame_system::Config>::AccountId, Perbill>;

    #[pallet::storage]
    #[pallet::getter(fn quota)]
    /// Quota acconting, transaction quota grown while account idle.
    pub(super) type Quota<T> = StorageMap<
        _,
        Twox64Concat,
        <T as frame_system::Config>::AccountId,
        (<<T as Config>::Time as Time>::Moment, u64),
    >;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Authenticates the RWS staker and dispatches a free function call.
        ///
        /// The dispatch origin for this call must be _Signed_.
        ///
        /// # <weight>
        /// - Dependes of call method.
        /// - Basically this sould be free by concept.
        /// # </weight>
        #[pallet::weight((0, call.get_dispatch_info().class, Pays::No))]
        pub fn call(
            origin: OriginFor<T>,
            call: Box<<T as Config>::Call>,
        ) -> DispatchResultWithPostInfo {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;
            ensure!(Self::check_call(call.clone()), Error::<T>::BadCall);
            ensure!(Self::check_quota(sender.clone()), Error::<T>::NoQuota);

            let res =
                call.dispatch_bypass_filter(frame_system::RawOrigin::Signed(sender.clone()).into());
            Self::deposit_event(Event::NewCall(sender, res.map(|_| ()).map_err(|e| e.error)));
            res
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
        #[pallet::weight(0)]
        pub fn set_oracle(
            origin: OriginFor<T>,
            new: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <Oracle<T>>::put(T::Lookup::lookup(new)?);
            Ok(().into())
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
        #[pallet::weight(0)]
        pub fn set_bandwidth(
            origin: OriginFor<T>,
            source: <T::Lookup as StaticLookup>::Source,
            share: Perbill,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(
                Some(sender) == <Oracle<T>>::get(),
                Error::<T>::OracleOnlyCall
            );
            <Bandwidth<T>>::insert(T::Lookup::lookup(source)?, share);
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Check staker quota for execute call.
        fn check_quota(staker: T::AccountId) -> bool {
            if let Some(share) = <Bandwidth<T>>::get(staker.clone()) {
                let now = T::Time::now();

                if let Some((last_active, points)) = <Quota<T>>::get(staker.clone()) {
                    let delta = now - last_active;
                    let new_points =
                        Self::estimate_points(share, delta.saturated_into::<u64>(), points);
                    if new_points >= CALL_COST {
                        // Enough quota points for the call, permit to call.
                        <Quota<T>>::insert(staker, (now, new_points - CALL_COST));
                        return true;
                    }
                } else {
                    // Quota points initialized, permit to call one time.
                    <Quota<T>>::insert(staker, (now, 0));
                    return true;
                }
            }
            // Cancel execution by default
            false
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
}

#[cfg(test)]
mod tests {
    use crate::{self as rws, *};
    use frame_support::{assert_err, assert_ok, parameter_types, weights::Weight};
    use pallet_robonomics_datalog as datalog;
    use sp_core::H256;
    use sp_runtime::{testing::Header, traits::IdentityLookup, DispatchError, Perbill};

    type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
    type Block = frame_system::mocking::MockBlock<Runtime>;

    frame_support::construct_runtime!(
        pub enum Runtime where
            Block = Block,
            NodeBlock = Block,
            UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system::{Module, Call, Config, Storage, Event<T>},
            Timestamp: pallet_timestamp::{Module, Storage},
            Datalog: datalog::{Module, Call, Storage, Event<T>},
            RWS: rws::{Module, Call, Storage, Event<T>},
        }
    );

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
        type Event = Event;
        type BlockHashCount = BlockHashCount;
        type Version = ();
        type AccountData = ();
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type DbWeight = ();
        type BaseCallFilter = ();
        type SystemWeightInfo = ();
        type BlockWeights = ();
        type BlockLength = ();
        type SS58Prefix = ();
        type PalletInfo = PalletInfo;
    }

    parameter_types! {
        pub const MinimumPeriod: u64 = 5;
    }

    impl pallet_timestamp::Config for Runtime {
        type Moment = u64;
        type OnTimestampSet = ();
        type MinimumPeriod = ();
        type WeightInfo = ();
    }
    parameter_types! {
        pub const WindowSize: u64 = 128;
        pub const MaximumMessageSize: usize = 512;
    }

    impl datalog::Config for Runtime {
        type Record = bool;
        type Event = Event;
        type Time = Timestamp;
        type WindowSize = WindowSize;
        type MaximumMessageSize = MaximumMessageSize;
        type WeightInfo = ();
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
        type Time = Timestamp;
        type Event = Event;
        type Call = Call;
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();
        storage.into()
    }

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
            assert_eq!(RWS::oracle(), Some(oracle));
        })
    }

    #[test]
    fn test_set_bandwidth() {
        let oracle = 1;
        let alice = 2;
        new_test_ext().execute_with(|| {
            assert_ok!(RWS::set_oracle(Origin::root(), oracle));

            assert_err!(
                RWS::set_bandwidth(Origin::none(), alice, Default::default()),
                DispatchError::BadOrigin,
            );

            assert_ok!(RWS::set_bandwidth(
                Origin::signed(oracle),
                alice,
                Perbill::from_percent(1),
            ));
            assert_eq!(RWS::bandwidth(alice), Some(Perbill::from_percent(1)));
        })
    }

    #[test]
    fn test_transaction() {
        let oracle = 1;
        let alice = 2;

        new_test_ext().execute_with(|| {
            Timestamp::set_timestamp(1600438152000);

            assert_ok!(RWS::set_oracle(Origin::root(), oracle));

            let call = Call::from(datalog::Call::record(true));

            assert_eq!(RWS::quota(oracle), None);
            assert_err!(
                RWS::call(Origin::signed(oracle), call.clone().into()),
                Error::<Runtime>::NoQuota,
            );

            assert_ok!(RWS::set_bandwidth(
                Origin::signed(oracle),
                alice,
                Perbill::from_percent(1),
            ),);
            assert_eq!(RWS::quota(alice), None);
            assert_ok!(RWS::call(Origin::signed(alice), call.clone().into()));
            assert_eq!(RWS::quota(alice), Some((1600438152000, 0)));

            Timestamp::set_timestamp(1600438156000);

            assert_ok!(RWS::call(Origin::signed(alice), call.clone().into()));
            assert_eq!(RWS::quota(alice), Some((1600438156000, 3 * CALL_COST)));

            assert_ok!(RWS::call(Origin::signed(alice), call.clone().into()));
            assert_eq!(RWS::quota(alice), Some((1600438156000, 2 * CALL_COST)));

            assert_ok!(RWS::call(Origin::signed(alice), call.clone().into()));
            assert_eq!(RWS::quota(alice), Some((1600438156000, 1 * CALL_COST)));

            assert_ok!(RWS::call(Origin::signed(alice), call.clone().into()));
            assert_eq!(RWS::quota(alice), Some((1600438156000, 0)));

            assert_err!(
                RWS::call(Origin::signed(alice), call.into()),
                Error::<Runtime>::NoQuota,
            );
        })
    }
}
