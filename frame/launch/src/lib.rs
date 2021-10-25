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
//! Simple robot launch runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {

    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Robot launch parameter data type.
        //type Parameter: Parameter + Default + From<Vec<u8>>;
        type Parameter: Parameter + Default + From<bool>;
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::storage]
	pub type Goal<T> = StorageValue<_, u32>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", T::Parameter = "LaunchParameter")]
    
    pub enum Event<T: Config> {
        /// Launch a robot with given parameter: sender, robot, parameter.
        NewLaunch(T::AccountId, T::AccountId, T::Parameter),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Launch a robot with given parameter.
        #[pallet::weight(500_000)]
        pub fn launch(
            origin: OriginFor<T>,
            robot: T::AccountId,
            param: T::Parameter,
            goal: u32,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            <Goal<T>>::put(goal); // Update storage
            Self::deposit_event(Event::NewLaunch(sender, robot, param));
            Ok(().into())
        }
    }
}

#[cfg(test)]
mod tests {
    use base58::FromBase58;
    use frame_support::{assert_err, assert_ok, parameter_types};
    use sp_core::H256;
    use sp_runtime::{testing::Header, traits::IdentityLookup, DispatchError};

    use crate::{self as launch, *};

    type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
    type Block = frame_system::mocking::MockBlock<Runtime>;
    type Item = RingBufferItem<Runtime>;
    type RuntimeError = Error<Runtime>;

    frame_support::construct_runtime!(
        pub enum Runtime where
            Block = Block,
            NodeBlock = Block,
            UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
            Timestamp: pallet_timestamp::{Pallet, Storage},
            Launch: launch::{Pallet, Call, Event<T>},
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
        type PalletInfo = PalletInfo;
        type AccountData = ();
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type DbWeight = ();
        type BaseCallFilter = ();
        type SystemWeightInfo = ();
        type BlockWeights = ();
        type BlockLength = ();
        type SS58Prefix = ();
        type OnSetCode = ();
    }

    impl pallet_timestamp::Config for Runtime {
        type Moment = u64;
        type OnTimestampSet = ();
        type MinimumPeriod = ();
        type WeightInfo = ();
    }

    const WINDOW: u64 = 20;
    parameter_types! {
        pub const WindowSize: u64 = WINDOW;
        pub const MaximumMessageSize: usize = 512;
    }

    impl Config for Runtime {
        type Time = Timestamp;
        type Parameter = bool;
        type Event = Event;

        type WindowSize = WindowSize;
        type MaximumMessageSize = MaximumMessageSize;
        type WeightInfo = ();
    }

    pub fn new_test_ext() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();
        storage.into()
    }

    #[test]
    fn test_store_data() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let param = true;
            assert_ok!(Launch::record(Origin::signed(sender), param.clone()));
        })
    }
}
