//! The Robonomics substrate module. This can be compiled with `#[no_std]`, ready for Wasm.

use rstd::prelude::Vec;
use runtime_primitives::traits::*;
use srml_support::{StorageValue, StorageMap, dispatch::Result};
use {balances, system::{self, ensure_signed}};

pub trait Trait: balances::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// An offer message.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Offer<Balance, AccountId> {
    model: Vec<u8>,
    objective: Vec<u8>,
    cost: Balance,
    sender: AccountId
}

/// An offer message.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Demand<Balance,AccountId> {
    model: Vec<u8>,
    objective: Vec<u8>,
    cost: Balance,
    sender: AccountId
}

/// An offer message.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Liability<Balance, AccountId> {
    model: Vec<u8>,
    objective: Vec<u8>,
    cost: Balance,
    promisee: AccountId,
    promisor: AccountId
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        /// Send a demand message
        pub fn demand(
            origin,
            model: Vec<u8>,
            objective: Vec<u8>,
            cost: T::Balance
        ) -> Result {
            // Ensure we have a signed message, and derive the sender's account id from the signature
            let sender = ensure_signed(origin)?;
            let lot = (model.clone(), objective.clone(), cost);
            let demand_hash = T::Hashing::hash_of(&lot);
            let demand = Demand { model, objective, cost, sender };

            Self::deposit_event(RawEvent::NewDemand(demand_hash));

            match Self::offer_of(demand_hash) {
                None => <DemandOf<T>>::insert(demand_hash, demand),
                Some(offer) => Self::create_liability(demand, offer),
            }

            Ok(())
        }
        
        /// Send a offer message
        pub fn offer(
            origin,
            model: Vec<u8>,
            objective: Vec<u8>,
            cost: T::Balance
        ) -> Result {
            // Ensure we have a signed message, and derive the sender's account id from the signature
            let sender = ensure_signed(origin)?;
            let lot = (model.clone(), objective.clone(), cost);
            let offer_hash = T::Hashing::hash_of(&lot);
            let offer = Offer { model, objective, cost, sender };

            Self::deposit_event(RawEvent::NewOffer(offer_hash)); 

            match Self::demand_of(offer_hash) {
                None => <OfferOf<T>>::insert(offer_hash, offer),
                Some(demand) => Self::create_liability(demand, offer),
            }

            Ok(())
        }

        /// Finalize liability
        pub fn finalize(origin, _result: Vec<u8>) -> Result {
            // Ensure we have a signed message, and derive the sender's account id from the signature
            let _sender = ensure_signed(origin)?;

            Ok(())
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Robonomics {
        pub DemandOf get(demand_of):
            map T::Hash => Option<Demand<T::Balance,T::AccountId>>; 

        pub OfferOf get(offer_of):
            map T::Hash => Option<Offer<T::Balance,T::AccountId>>;

        /// The (hashes of) active liabilities.
        pub LiabilityCount get(liability_count) config(): u64 = 0;

        /// Actual liability by id.
        pub LiabilityOf get(liability_of):
            map u64 => Option<Liability<T::Balance,T::AccountId>>;

    }
}

/// Event for this module.
decl_event! {
    pub enum Event<T> where <T as system::Trait>::Hash, <T as system::Trait>::AccountId
    {
        /// A Demand was published.
        NewDemand(Hash),

        /// An Offer was published.
        NewOffer(Hash),

        NewLiability(u64, AccountId, AccountId),

        Finalized(u64),
    }
}

impl<T: Trait> Module<T> {
    fn create_liability(demand: Demand<T::Balance,T::AccountId>, offer: Offer<T::Balance,T::AccountId>) {
        let Demand { model, objective, cost, sender: promisee } = demand;
        let Offer { model: _m, objective: _o, cost: _c, sender: promisor } = offer;

        let id = Self::liability_count();
        Self::deposit_event(RawEvent::NewLiability(id, promisee.clone(), promisor.clone()));

        let liability = Liability { model, objective, cost, promisee, promisor };
        <LiabilityOf<T>>::insert(id, liability);
        <LiabilityCount<T>>::put(id + 1);
    }
}
