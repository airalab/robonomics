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
//! Simple robot launch runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, EncodeLike};
use frame_support::{decl_event, decl_module, decl_storage};
use frame_system::ensure_signed;
use sp_runtime::traits::Member;
use sp_std::prelude::*;

/// Launch module main trait.
pub trait Trait: frame_system::Trait {
    /// Robot launch parameter data type.
    type Parameter: Codec + EncodeLike + Member;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

/// Robot launch XCMP message trait.
pub trait LaunchMessage<AccountId, Parameter>: Sized {
    /// Launch robot with given launch parameter.
    fn launch(from: AccountId, to: AccountId, param: Parameter) -> Self;
}

decl_event! {
    pub enum Event<T>
    where AccountId = <T as frame_system::Trait>::AccountId,
          Parameter = <T as Trait>::Parameter,
    {
        /// Launch a robot with given parameter: sender, robot, parameter.
        NewLaunch(AccountId, AccountId, Parameter),
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Launch {}
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Launch a robot with given parameter.
        #[weight = 5_000_000]
        fn launch(origin, robot: T::AccountId, param: T::Parameter) {
            let sender = ensure_signed(origin)?;
            Self::deposit_event(RawEvent::NewLaunch(sender, robot, param));
        }
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

    impl Trait for Runtime {
        type Parameter = bool;
        type Event = ();
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();
        storage.into()
    }

    type Launch = Module<Runtime>;

    #[test]
    fn test_launch() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let robot = 42;
            assert_ok!(Launch::launch(Origin::signed(sender), robot, true));
        })
    }

    #[test]
    fn test_bad_origin() {
        new_test_ext().execute_with(|| {
            assert_err!(
                Launch::launch(Origin::none(), 0, false),
                DispatchError::BadOrigin
            );
        })
    }
}
