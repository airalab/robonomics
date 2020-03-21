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
//! Simple Robonomics datalog runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{EncodeLike, Codec};
use sp_std::prelude::*;
use sp_runtime::traits::Member;
use frame_system::{self as system, ensure_signed, offchain::SubmitSignedTransaction};
use frame_support::{
    decl_module, decl_storage, decl_event, decl_error,
    weights::SimpleDispatchInfo, traits::Time,
};

/// Type synonym for timestamp data type.
pub type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;

/// Datalog module main trait.
pub trait Trait: system::Trait {
    /// Timestamp source.
    type Time: Time;
    /// Datalog record data type.
    type Record: Codec + EncodeLike + Member;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

/// Datalog module agent trait.
pub trait Agent: Trait {
    type Call: From<Call<Self>>;
    type SubmitTransaction: SubmitSignedTransaction<Self, <Self as Agent>::Call>;
}

decl_event! {
    pub enum Event<T>
    where AccountId = <T as system::Trait>::AccountId,
          Moment = MomentOf<T>,
          Record = <T as Trait>::Record,
    {
        /// New data added. 
        NewRecord(AccountId, Moment, Record),
        /// Account datalog erased.
        Erased(AccountId),
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Datalog {
        /// Time tagged data of given account.
        Datalog get(fn datalog): map hasher(blake2_128_concat)
                                 T::AccountId => Vec<(MomentOf<T>, T::Record)>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Store new data into blockchain.
        #[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
        fn record(origin, record: T::Record) {
            let sender = ensure_signed(origin)?;
            let now = T::Time::now();
            <Datalog<T>>::mutate(
                sender.clone(),
                |m| m.push((now.clone(), record.clone()))
            );
            Self::deposit_event(RawEvent::NewRecord(sender, now, record));
        }

        /// Clear account datalog.
        #[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
        fn erase(origin) {
            let sender = ensure_signed(origin)?;
            <Datalog<T>>::remove(sender.clone());
            Self::deposit_event(RawEvent::Erased(sender));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use sp_core::H256;
    use base58::FromBase58;
    use node_primitives::Moment;
    use frame_system::{self as system};
    use sp_runtime::{
        DispatchError, Perbill,
        traits::IdentityLookup, testing::Header,
    };
    use frame_support::{
        impl_outer_origin, parameter_types,
        assert_ok, assert_err,
        weights::Weight,
    };

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
    }

    parameter_types! {
        pub const MinimumPeriod: Moment = 5;
    }

    impl pallet_timestamp::Trait for Runtime {
        type Moment = Moment;
        type OnTimestampSet = ();
        type MinimumPeriod = ();
    }

    impl Trait for Runtime {
        type Time = Timestamp;
        type Record = Vec<u8>;
        type Event = ();
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();
        storage.into()
    }

    type Timestamp = pallet_timestamp::Module<Runtime>;
    type Datalog = Module<Runtime>;

    #[test]
    fn test_store_data() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let record = vec![42];
            assert_ok!(Datalog::record(Origin::signed(sender), record.clone()));
            assert_eq!(Datalog::datalog(sender), vec![(0, record)]);
        })
    }

    #[test]
    fn test_erase_data() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let record = vec![1,2,3];
            assert_ok!(Datalog::record(Origin::signed(sender), record.clone()));
            assert_eq!(Datalog::datalog(sender), vec![(0, record)]);
            assert_ok!(Datalog::erase(Origin::signed(sender)));
            assert_eq!(Datalog::datalog(sender).is_empty(), true);
        })
    }

    #[test]
    fn test_bad_origin() {
        new_test_ext().execute_with(|| {
            assert_err!(Datalog::record(Origin::NONE, vec![]), DispatchError::BadOrigin);
        })
    }

    #[test]
    fn test_store_ipfs_hashes() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let record = "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz4".from_base58().unwrap();
            assert_ok!(Datalog::record(Origin::signed(sender), record.clone()));
            assert_eq!(Datalog::datalog(sender), vec![(0, record.clone())]);
            let record2 = "zdj7WWYAEceQ6ncfPZeRFjozov4dC7FaxU7SuMwzW4VuYBDta".from_base58().unwrap();
            assert_ok!(Datalog::record(Origin::signed(sender), record2.clone()));
            assert_eq!(Datalog::datalog(sender), vec![
                       (0, record.clone()),
                       (0, record2.clone()),
            ]);
            let record3 = "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz2".from_base58().unwrap();
            assert_ok!(Datalog::record(Origin::signed(sender), record3.clone()));
            assert_eq!(Datalog::datalog(sender), vec![
                       (0, record),
                       (0, record2),
                       (0, record3),
            ]);
        })
    }
}
