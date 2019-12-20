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

use codec::{Encode, Decode};
use sp_std::{
    prelude::*,
    collections::btree_map::BTreeMap,
};
use sp_runtime::{RuntimeDebug, traits::Hash};
use support::{
    decl_module, decl_event, decl_storage, debug, StorageValue, 
    weights::SimpleDispatchInfo,
    dispatch::Result,
};
use system::{ensure_signed, offchain::SubmitUnsignedTransaction};
use liability::{
    TechnicalParam, EconomicalParam, ProofParam, AccountId,
    traits::{Agreement, ProofTarget},
};

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
pub trait Trait: liability::Trait {
    /// A dispatchable call type. We need to define it for the offchain worker
    type Call: From<Call<Self>>;

    /// Let's define the helper we use to create signed transactions with
    type SubmitTransaction: SubmitUnsignedTransaction<Self, <Self as Trait>::Call>;

    /// AccountId convertion
    type Account: Into<AccountId<Self>> + From<<Self as system::Trait>::AccountId>;

    /// The regular events type
    type Event:From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// The type of requests we can send to the offchain worker
#[cfg_attr(feature = "std", derive(PartialEq, Eq))]
#[derive(Encode, Decode, RuntimeDebug)]
pub enum OffchainRequest<T: liability::Trait> {
    Demand(TechnicalParam<T>, EconomicalParam<T>, ProofParam<T>, AccountId<T>),
    Offer(TechnicalParam<T>, EconomicalParam<T>, ProofParam<T>, AccountId<T>),
}

#[cfg_attr(feature = "std", derive(PartialEq, Eq))]
#[derive(Encode, Decode, RuntimeDebug)]
pub struct Order<T: Trait> {
    technics: TechnicalParam<T>,
    economics: EconomicalParam<T>,
    proof: ProofParam<T>,
    sender: AccountId<T>,
}

#[derive(Encode, Decode, RuntimeDebug)]
pub struct WorkerState<T: Trait> {
    demand_of:   BTreeMap<T::Hash, Vec<Order<T>>>,
    offer_of:    BTreeMap<T::Hash, Vec<Order<T>>>,
    last_update: T::BlockNumber,
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

        /// Send service demand request to network
		#[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
        fn demand(
            origin,
            technics:  TechnicalParam<T>,
            economics: EconomicalParam<T>,
            proof:     ProofParam<T>,
        ) -> Result {
            let sender = T::Account::from(ensure_signed(origin)?).into();
            let liability = T::Liability::new(
                technics.clone(),
                economics.clone(),
                sender.clone(),
                sender.clone()
            );

            if !liability.verify(ProofTarget::Promisee, &proof) {
                Err("Bad signature")
            } else {
                <OcRequests<T>>::mutate(|v|
                    v.push(OffchainRequest::Demand(technics, economics, proof, sender))
                );
                Ok(())
            }
        }

        /// Send service offer request to network 
		#[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
        fn offer(
            origin,
            technics:  TechnicalParam<T>,
            economics: EconomicalParam<T>,
            proof:     ProofParam<T>,
        ) -> Result {
            let sender = T::Account::from(ensure_signed(origin)?).into();
            let liability = T::Liability::new(
                technics.clone(),
                economics.clone(),
                sender.clone(),
                sender.clone(),
            );

            if !liability.verify(ProofTarget::Promisee, &proof) {
                Err("Bad signature")
            } else {
                <OcRequests<T>>::mutate(|v|
                    v.push(OffchainRequest::Offer(technics, economics, proof, sender))
                );
                Ok(())
            }
        }

        // Runs after every block within the context and current state of said block.
        fn offchain_worker(now: T::BlockNumber) {
            debug::RuntimeLogger::init();

            if sp_io::offchain::is_validator() {
                Self::offchain(now);
            }
        }
    }
}

impl<T: Trait> Module<T> {
    /// The main entry point
    fn offchain(now: T::BlockNumber) {
        for e in <OcRequests<T>>::get() {
            match e {
                OffchainRequest::Demand(technics, economics, proof, sender) => {
                    let order = Order::<T>{ technics, economics, proof, sender };
                    let order_id: T::Hash = T::Hashing::hash_of(&order);
                    debug::info!(
                        target: "robonomics-provider",
                        "Get demand {:?} from {:?}", order_id, order.sender
                    );
                },
                OffchainRequest::Offer(technics, economics, proof, sender) => {
                    let order = Order::<T>{ technics, economics, proof, sender };
                    let order_id: T::Hash = T::Hashing::hash_of(&order);
                    debug::info!(
                        target: "robonomics-provider",
                        "Get offer {:?} from {:?}", order_id, order.sender
                    );
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
    use sp_runtime::{traits::{Verify, IdentifyAccount}};
    use offchain::testing::TestOffchainExt;
    use node_primitives::{AccountId, AccountIndex, Signature};
    use primitives::{offchain, H256, sr25519, crypto::Pair};

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

    impl liability::Trait for Runtime {
        type Event = ();
        type Technics = liability::technics::PureIPFS;
        type Economics = liability::economics::Communism;
        type Liability = liability::signed::SignedLiability<
            Self::Technics,
            Self::Economics,
            <Signature as Verify>::Signer,
            Signature,
        >;
    }

    impl Trait for Runtime {
        type Event = ();
        type Call = Call;
        type Account = AccountId;
        type SubmitTransaction = system::offchain::TransactionSubmitter<(), Call, Extrinsic>;
    }

    type System = system::Module<Runtime>;
    type Provider = Module<Runtime>;

    pub fn new_test_ext() -> sp_io::TestExternalities {
        let t = system::GenesisConfig::default().build_storage::<Runtime>().unwrap();
        t.into()
    }

    #[test]
    fn test_demand_request() {
        new_test_ext().execute_with(|| {
            let pair: sr25519::Pair = Pair::from_string("//Alice", None).unwrap();
            let sender = <Signature as Verify>::Signer::from(pair.public()).into_account();
            let technics = vec![1,2,3];
            let economics = ();
            let order = (technics.clone(), economics.clone());
            let proof = order.using_encoded(|params| pair.sign(params));
            assert_ok!(Provider::demand(Origin::signed(sender), technics, economics, proof.into()));
            assert_eq!(Provider::oc_requests().len(), 1);
        })
    }

    #[test]
    fn test_offer_request() {
        new_test_ext().execute_with(|| {
            let pair: sr25519::Pair = Pair::from_string("//Alice", None).unwrap();
            let sender = <Signature as Verify>::Signer::from(pair.public()).into_account();
            let technics = vec![1,2,3];
            let economics = ();
            let order = (technics.clone(), economics.clone());
            let proof = order.using_encoded(|params| pair.sign(params));
            assert_ok!(Provider::offer(Origin::signed(sender), technics, economics, proof.into()));
            assert_eq!(Provider::oc_requests().len(), 1);
        })
    }
}
