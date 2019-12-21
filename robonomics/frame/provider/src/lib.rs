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
};
use primitives::offchain::StorageKind;
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
const DB_KEY: &[u8] = b"airalab/robonomics-provider-worker";

/// The module's main configuration trait.
pub trait Trait: liability::Trait {
    /// A dispatchable call type.
    type Call: From<liability::Call<Self>>;

    /// Let's define the helper we use to create signed transactions with
    type SubmitTransaction: SubmitUnsignedTransaction<Self, <Self as Trait>::Call>;

    /// The regular events type
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// The type of requests we can send to the offchain worker
#[cfg_attr(feature = "std", derive(PartialEq, Eq))]
#[derive(Encode, Decode, RuntimeDebug)]
pub enum OffchainRequest<T: liability::Trait> {
    Demand(TechnicalParam<T>, EconomicalParam<T>, ProofParam<T>, AccountId<T>),
    Offer(TechnicalParam<T>, EconomicalParam<T>, ProofParam<T>, AccountId<T>),
}

#[cfg_attr(feature = "std", derive(PartialEq, Eq))]
#[derive(Encode, Decode, Clone, RuntimeDebug)]
pub struct Order<T: Trait> {
    technics:  TechnicalParam<T>,
    economics: EconomicalParam<T>,
    proof:     ProofParam<T>,
    sender:    AccountId<T>,
}

#[derive(Encode, Decode, Clone, RuntimeDebug)]
pub struct WorkerState<T: Trait> where <T as system::Trait>::Hash: Ord {
    demand_of:   BTreeMap<T::Hash, Vec<Order<T>>>,
    offer_of:    BTreeMap<T::Hash, Vec<Order<T>>>,
    last_update: T::BlockNumber,
}

decl_event!(
    pub enum Event<T>
    where AccountId = AccountId<T>,
          Technics  = TechnicalParam<T>,
          Economics = EconomicalParam<T>,
    {
        NewDemand(Technics, Economics, AccountId),
        NewOffer(Technics, Economics, AccountId),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as Provider where <T as system::Trait>::Hash: Ord {
        /// Requests made within this block execution
        OcRequests get(oc_requests): Vec<OffchainRequest<T>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin, <T as system::Trait>::Hash: Ord {
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
        ) {
            let sender = ensure_signed(origin)?;
            let liability = T::Liability::new(
                technics.clone(),
                economics.clone(),
                sender.clone(),
                sender.clone()
            );

            if !liability.verify(ProofTarget::Promisee, &proof) {
                Err("Bad signature")?
            }

            Self::deposit_event(RawEvent::NewDemand(technics.clone(), economics.clone(), sender.clone()));
            <OcRequests<T>>::mutate(|requests|
                requests.push(OffchainRequest::Demand(technics, economics, proof, sender))
            );
        }

        /// Send service offer request to network 
        #[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
        fn offer(
            origin,
            technics:  TechnicalParam<T>,
            economics: EconomicalParam<T>,
            proof:     ProofParam<T>,
        ) {
            let sender = ensure_signed(origin)?;
            let liability = T::Liability::new(
                technics.clone(),
                economics.clone(),
                sender.clone(),
                sender.clone(),
            );

            if !liability.verify(ProofTarget::Promisee, &proof) {
                Err("Bad signature")?
            }

            Self::deposit_event(RawEvent::NewOffer(technics.clone(), economics.clone(), sender.clone()));
            <OcRequests<T>>::mutate(|requests|
                requests.push(OffchainRequest::Offer(technics, economics, proof, sender))
            );
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

impl<T: Trait> Module<T> where <T as system::Trait>::Hash: Ord {
    /// The main entry point
    fn offchain(now: T::BlockNumber) {
        for e in <OcRequests<T>>::get() {
            match e {
                OffchainRequest::Demand(technics, economics, proof, sender) => {
                    let params = (technics.clone(), economics.clone());
                    let order = Order::<T>{ technics, economics, proof, sender };
                    let order_id: T::Hash = T::Hashing::hash_of(&params);
                    debug::info!(
                        target: "robonomics-provider",
                        "Get demand params {:?} from {:?}", order_id, order.sender
                    );

                    // Match offer by order id
                    if let Some(offer) = Self::get_offer(order_id, now) {
                        debug::info!(
                            target: "robonomics-provider",
                            "Matched {:?} with {:?}", order.sender, offer.sender
                        );
                        let call = liability::Call::<T>::create(
                            order.technics,
                            order.economics,
                            order.sender,
                            offer.sender,
                            order.proof,
                            offer.proof,
                        );
                        let res = T::SubmitTransaction::submit_unsigned(call);
                        debug::info!(
                            target: "robonomics-provider",
                            "Call {:?}", res
                        );
                    } else {
                        debug::info!(
                            target: "robonomics-provider",
                            "Not matched"
                        );
                        Self::put_demand(order_id, order, now);
                    }
                },

                OffchainRequest::Offer(technics, economics, proof, sender) => {
                    let params = (technics.clone(), economics.clone());
                    let order = Order::<T>{ technics, economics, proof, sender };
                    let order_id: T::Hash = T::Hashing::hash_of(&params);
                    debug::info!(
                        target: "robonomics-provider",
                        "Get offer params {:?} from {:?}", order_id, order.sender
                    );

                    // Match demand by order id
                    if let Some(demand) = Self::get_demand(order_id, now) {
                        debug::info!(
                            target: "robonomics-provider",
                            "Matched {:?} with {:?}", order.sender, demand.sender
                        );
                        let call = liability::Call::<T>::create(
                            order.technics,
                            order.economics,
                            demand.sender,
                            order.sender,
                            demand.proof,
                            order.proof,
                        );
                        let res = T::SubmitTransaction::submit_unsigned(call);
                        debug::info!(
                            target: "robonomics-provider",
                            "Call {:?}", res
                        );
                    } else {
                        debug::info!(
                            target: "robonomics-provider",
                            "Not matched"
                        );
                        Self::put_offer(order_id, order, now);
                    }
                }
            }
        }
    }

    fn storage_mutate<A>(now: T::BlockNumber, f: impl Fn(WorkerState<T>) -> (WorkerState<T>, A)) -> A {
        Self::storage(|state| {
            let (mut new_state, result) = f(state);
            new_state.last_update = now;
            sp_io::offchain::local_storage_set(
                StorageKind::PERSISTENT,
                DB_KEY,
                &new_state.encode()
            );
            result
        })
    }

    fn storage<A>(f: impl Fn(WorkerState<T>) -> A) -> A {
        f(
            if let Some(db) = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, DB_KEY) {
                if let Ok(state) = WorkerState::decode(&mut &db[..]) { state }
                else { WorkerState {
                    demand_of: BTreeMap::new(),
                    offer_of: BTreeMap::new(),
                    last_update: T::BlockNumber::default(),
                } }
            } else { WorkerState {
                demand_of: BTreeMap::new(),
                offer_of: BTreeMap::new(),
                last_update: T::BlockNumber::default(),
            } }
        )
    }

    fn get_demand(order_id: <T as system::Trait>::Hash, now: T::BlockNumber) -> Option<Order<T>> {
        Self::storage_mutate(now, |state| {
            let mut new_state = state.clone();
            let result = match new_state.demand_of.get_mut(&order_id) {
                Some(list) => list.pop(),
                None => None
            };
            (new_state, result)
        })
    }

    fn get_offer(order_id: <T as system::Trait>::Hash, now: T::BlockNumber) -> Option<Order<T>> {
        Self::storage_mutate(now, |state| {
            let mut new_state = state.clone();
            let result = match new_state.offer_of.get_mut(&order_id) {
                Some(list) => list.pop(),
                None => None
            };
            (new_state, result)
        })
    }

    fn put_demand(order_id: <T as system::Trait>::Hash, order: Order<T>, now: T::BlockNumber) {
        Self::storage_mutate(now, |state| {
            let mut new_state = state.clone();
            match new_state.demand_of.get_mut(&order_id) {
                Some(list) => list.push(order.clone()),
                None => { new_state.demand_of.insert(order_id, vec![order.clone()]); }
            };
            (new_state, ())
        })
    }

    fn put_offer(order_id: <T as system::Trait>::Hash, order: Order<T>, now: T::BlockNumber) {
        Self::storage_mutate(now, |state| {
            let mut new_state = state.clone();
            match new_state.offer_of.get_mut(&order_id) {
                Some(list) => list.push(order.clone()),
                None => { new_state.offer_of.insert(order_id, vec![order.clone()]); }
            };
            (new_state, ())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as provider;
    use sp_runtime::{
        Perbill, generic,
        testing::{Header, TestXt},
        traits::{IdentityLookup, BlakeTwo256},
    };
    use support::{
        impl_outer_event,
        impl_outer_origin,
        impl_outer_dispatch,
        parameter_types,
        assert_ok
    };
    use sp_runtime::{traits::{Verify, IdentifyAccount}};
    use node_primitives::{AccountId, AccountIndex, Signature};
    use primitives::{
        offchain::{
            OffchainExt, TransactionPoolExt,
            testing::{TestOffchainExt, TestTransactionPoolExt},
        },
        H256, sr25519, crypto::Pair
    };

    impl_outer_event! {
        pub enum MetaEvent for Runtime {
            liability<T>, provider<T>,
        }
    }

    impl_outer_origin!{
        pub enum Origin for Runtime {}
    }

    impl_outer_dispatch! {
        pub enum Call for Runtime where origin: Origin {
            system::System,
            liability::Liability,
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
        type Event = MetaEvent;
        type BlockHashCount = BlockHashCount;
        type MaximumBlockWeight = MaximumBlockWeight;
        type MaximumBlockLength = MaximumBlockLength;
        type AvailableBlockRatio = AvailableBlockRatio;
        type Version = ();
    }

    impl liability::Trait for Runtime {
        type Event = MetaEvent;
        type Technics = liability::technics::PureIPFS;
        type Economics = liability::economics::Communism;
        type Liability = liability::signed::SignedLiability<
            Self::Technics,
            Self::Economics,
            Signature,
            <Signature as Verify>::Signer,
            AccountId,
        >;
    }

    impl Trait for Runtime {
        type Event = MetaEvent;
        type Call = Call;
        type SubmitTransaction = system::offchain::TransactionSubmitter<(), Call, Extrinsic>;
    }

    type System = system::Module<Runtime>;
    type Liability = liability::Module<Runtime>;
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

    #[test]
    fn test_offchain_worker() {
        let mut ext = new_test_ext();
        let (offchain, _state) = TestOffchainExt::new();
        let (pool, state) = TestTransactionPoolExt::new();
        ext.register_extension(OffchainExt::new(offchain));
        ext.register_extension(TransactionPoolExt::new(pool));
        ext.execute_with(|| {
            System::set_block_number(1);

            let pair: sr25519::Pair = Pair::from_string("//Alice", None).unwrap();
            let sender = <Signature as Verify>::Signer::from(pair.public()).into_account();
            let technics = vec![1,2,3];
            let economics = ();
            let order = (technics.clone(), economics.clone());
            let proof = order.using_encoded(|params| pair.sign(params));
            assert_ok!(Provider::offer(Origin::signed(sender), technics, economics, proof.into()));

            let pair: sr25519::Pair = Pair::from_string("//Bob", None).unwrap();
            let sender = <Signature as Verify>::Signer::from(pair.public()).into_account();
            let technics = vec![1,2,3];
            let economics = ();
            let order = (technics.clone(), economics.clone());
            let proof = order.using_encoded(|params| pair.sign(params));
            assert_ok!(Provider::demand(Origin::signed(sender), technics, economics, proof.into()));
        })
    }
}
