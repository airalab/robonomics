///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2026 Robonomics Network <research@robonomics.network>
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

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

pub use pallet::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Robot launch parameter data type.
        type Parameter: Parameter + Default + MaxEncodedLen;
        /// The overarching event type.
        #[allow(deprecated)]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Extrinsic weights
        type WeightInfo: WeightInfo;
    }

    #[pallet::storage]
    pub type Goal<T> = StorageValue<_, <T as Config>::Parameter>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Launch a robot with given parameter: sender, robot, parameter.
        NewLaunch(T::AccountId, T::AccountId, T::Parameter),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Launch a robot with given parameter.
        #[pallet::weight(T::WeightInfo::launch())]
        #[pallet::call_index(0)]
        pub fn launch(
            origin: OriginFor<T>,
            robot: T::AccountId,
            param: T::Parameter,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            <Goal<T>>::put(param.clone()); // Update storage
            Self::deposit_event(Event::NewLaunch(sender, robot, param));
            Ok(().into())
        }
    }
}

#[cfg(test)]
mod tests {
    use frame_support::{assert_ok, derive_impl, parameter_types, BoundedVec};

    use sp_runtime::BuildStorage;

    use crate::{self as launch, *};

    type Block = frame_system::mocking::MockBlock<Runtime>;

    frame_support::construct_runtime!(
        pub enum Runtime {
            System: frame_system,
            Timestamp: pallet_timestamp,
            Launch: launch,
        }
    );

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
    }

    #[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
    impl frame_system::Config for Runtime {
        type Block = Block;
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
        pub const MaximumMessageSize: u32 = 512;
    }

    impl Config for Runtime {
        type Parameter = BoundedVec<u8, MaximumMessageSize>;
        type RuntimeEvent = RuntimeEvent;
        type WeightInfo = weights::TestWeightInfo;
    }

    pub fn new_test_ext() -> sp_io::TestExternalities {
        let storage = RuntimeGenesisConfig {
            system: Default::default(),
        }
        .build_storage()
        .unwrap();
        storage.into()
    }

    #[test]
    fn test_store_data() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let decoded = bs58::decode("QmY91yTMHzAd9csvKtPF1b1NS5CVhdoSRz2CBwTGTxkvST")
                .into_vec()
                .expect("Couldn't decode from Base58");
            let param = BoundedVec::try_from(decoded).expect("Bad bounds");
            let data = 0;
            assert_ok!(Launch::launch(RuntimeOrigin::signed(sender), data, param));
        })
    }
}
