///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2019 Airalab <research@aira.life> 
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
//! Robonomics Network provider module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// We have to import a few things
use sp_std::prelude::*;
use app_crypto::RuntimeAppPublic;
use sp_runtime::traits::Member;
use support::{
    decl_module, decl_event, decl_storage, debug, StorageValue, 
    dispatch::{Parameter, Result}
};
use system::{ensure_signed, ensure_root};
use system::offchain::SubmitUnsignedTransaction;
use codec::{Encode, Decode};

/// Provider crypto primitives.
// XXX: Currently unused.
//pub mod crypto;

/// Our local KeyType.
///
/// For security reasons the offchain worker doesn't have direct access to the keys
/// but only to app-specific subkeys, which are defined and grouped by their `KeyTypeId`.
/// We define it here as `xrtp` (XRT Provider).
// XXX: Currently unused.
//pub const KEY_TYPE: app_crypto::KeyTypeId = app_crypto::KeyTypeId(*b"xrtp");

/// The local storage database key under which the worker progress status
/// is tracked.
// XXX: Currently unused.
//const DB_KEY: &[u8] = b"airalab/robonomics-provider-worker";

/// The module's main configuration trait.
pub trait Trait: system::Trait  {
    /// A dispatchable call type. We need to define it for the offchain worker
    type Call: From<Call<Self>>;

    /// Let's define the helper we use to create signed transactions with
    type SubmitTransaction: SubmitUnsignedTransaction<Self, <Self as Trait>::Call>;

    /// The regular events type
    type Event:From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// The type of requests we can send to the offchain worker
#[cfg_attr(feature = "std", derive(PartialEq, Eq, Debug))]
#[derive(Encode, Decode)]
pub enum OffchainRequest<T: system::Trait> {
    Demand(T::AccountId),
    Offer(T::AccountId),
}

decl_event!(
    pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
        NewDemand(AccountId),
        NewOffer(AccountId),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as Provider {
        /// Requests made within this block execution
        OcRequests get(oc_requests): Vec<OffchainRequest<T>>;
        // The current set of keys that may create liabilities 
        //Providers get(providers) config(): Vec<T::AccountId>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// Initializing events
        fn deposit_event() = default;

        /// Clean the state on initialisation of a block
        fn on_initialize(_now: T::BlockNumber) {
            // At the beginning of each block execution, system triggers all
            // `on_initialize` functions, which allows us to set up some temporary state or - like
            // in this case - clean up other states
            <OcRequests<T>>::kill();
        }

        pub fn demand(origin) -> Result {
            let who = ensure_signed(origin)?;
            <OcRequests<T>>::mutate(|v| v.push(OffchainRequest::Demand(who)));
            Ok(())
        }

        pub fn offer(origin) -> Result {
            let who = ensure_signed(origin)?;
            <OcRequests<T>>::mutate(|v| v.push(OffchainRequest::Offer(who)));
            Ok(())
        }

        // Runs after every block within the context and current state of said block.
        fn offchain_worker(_now: T::BlockNumber) {
            debug::RuntimeLogger::init();
            Self::offchain();
        }
    }
}


// We've moved the  helper functions outside of the main declaration for brevity.
impl<T: Trait> Module<T> {
    /// The main entry point
    fn offchain() {
        for e in <OcRequests<T>>::get() {
            match e {
                OffchainRequest::Demand(who) => {
                    debug::info!(target: "xrtd", "Get demand from {:?}", who);
                }
                OffchainRequest::Offer(who) => {
                    debug::info!(target: "xrtd", "Get offer from {:?}", who);
                }
            }

            // TODO
            //let call = Call::<...>
            //T::SubmitTransaction::submit_unsigned(call);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codec::Decode;
    use std::sync::Arc;
    use sp_runtime::{
        Perbill, generic, RuntimeAppPublic,
        testing::{Header, TestXt, UintAuthorityId},
        traits::{IdentityLookup, BlakeTwo256, Block, Dispatchable},
    };
    use support::{impl_outer_origin, impl_outer_dispatch, parameter_types, assert_ok};
    use offchain::testing::TestOffchainExt;
    use primitives::{offchain, H256};
    use runtime_io;
    use system;

    impl_outer_origin!{
        pub enum Origin for Runtime {}
    }

    impl_outer_dispatch! {
        pub enum Call for Runtime where origin: Origin {
            provider::Provider,
        }
    }

    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct Runtime;

    // Define some type aliases. We use the simplest form of anything which is not relevant for
    // simplicity, e.g. account ids are just numbers and signed extensions are empty (`()`).
    type AccountId = u64;
    type AccountIndex = u64;
    type Extrinsic = TestXt<Call, ()>;
    type NodeBlock = generic::Block<Header, Extrinsic>;

    // Define the required constants for system module,
    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub const MaximumBlockWeight: u32 = 1024;
        pub const MaximumBlockLength: u32 = 2 * 1024;
        pub const AvailableBlockRatio: Perbill = Perbill::one();
    }

    // and add it to our test runtime.
    impl system::Trait for Runtime {
        type Origin = Origin;
        type Index = AccountIndex;
        type BlockNumber = u64;
        type Call = Call;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = AccountId;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = ();
        type BlockHashCount = BlockHashCount;
        type MaximumBlockWeight = MaximumBlockWeight;
        type MaximumBlockLength = MaximumBlockLength;
        type AvailableBlockRatio = AvailableBlockRatio;
        type Version = ();
    }

    impl Trait for Runtime {
        type Event = ();
        type Call = Call;
        type SubmitTransaction = system::offchain::TransactionSubmitter<(), Call, Extrinsic>;
    }

    type System = system::Module<Runtime>;
    type Provider = Module<Runtime>;

    pub fn new_test_ext() -> runtime_io::TestExternalities {
        let t = system::GenesisConfig::default().build_storage::<Runtime>().unwrap();
        t.into()
    }

    // Send a ping and verify that the ping struct has been stored in the `OcRequests` storage.
    #[test]
    fn demand_should_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Provider::demand(Origin::signed(1)));
            assert_eq!(Provider::oc_requests().len(), 1);
            assert_eq!(Provider::oc_requests()[0], OffchainRequest::Demand(1));
        })
    }

/*
    // Verify that any origin can send a ping and the even is triggered regardless.
    #[test]
    fn anyone_can_ping() {
        // Current node is an authority. This does not matter in this test.
        with_externalities(&mut new_test_ext(vec![49, 10]), || {
            // An authority (current node) can submit ping.
            assert_ok!(OffchainCb::ping(Origin::signed(49), 1));
            // normal key can also submit ping.
            assert_ok!(OffchainCb::ping(Origin::signed(10), 4));

            // both should be processed.
            assert_eq!(
                OffchainCb::oc_requests()[0],
                offchaincb::OffchainRequest::Ping(1, 49),
            );

            assert_eq!(
                OffchainCb::oc_requests()[1],
                offchaincb::OffchainRequest::Ping(4, 10),
            );
        })
    }

    // Verify that the offchain is executed if the current node is an authority.
    #[test]
    fn ping_triggers_ack() {
        // Assume current node has key 49, hence is an authority.
        let mut ext = new_test_ext(vec![49]);
        let (offchain, state) = TestOffchainExt::new();
        ext.set_offchain_externalities(offchain);

        with_externalities(&mut ext, || {
            // 2 submits a ping. Assume this is an extrinsic from the outer world.
            assert_ok!(OffchainCb::ping(Origin::signed(2), 1));
            assert_eq!(
                OffchainCb::oc_requests()[0],
                offchaincb::OffchainRequest::Ping(1, 2),
            );

            // 49 is an authority (current externality), should be able to call pong.
            assert!(seal_block(1, state).is_some());

            // which triggers ack
            assert_eq!(
                System::events()[0].event,
                Event::offchaincb(offchaincb::RawEvent::Ack(1, 49)),
            );
        })
    }

    // Verify that a non-authority will not execute the offchain logic.
    #[test]
    fn only_authorities_can_pong() {
        // Current node does not have key 49, hence is not the authority.
        let mut ext = new_test_ext(vec![69]);
        let (offchain, state) = TestOffchainExt::new();
        ext.set_offchain_externalities(offchain);

        with_externalities(&mut ext, || {
            assert_ok!(OffchainCb::ping(Origin::signed(2), 1));
            // 69 is not an authority.
            assert!(seal_block(1, state).is_none());
        })
    }
*/
}
