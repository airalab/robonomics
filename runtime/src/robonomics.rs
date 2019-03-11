//! The Robonomics substrate module. This can be compiled with `#[no_std]`, ready for Wasm.

use rstd::vec::Vec;
use runtime_primitives::traits::*;
use srml_support::{StorageValue, StorageMap, dispatch::Result};
use {balances, system::{self, ensure_signed}};

/// Offer message.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Offer<Balance, AccountId> {
    model: Vec<u8>,
    objective: Vec<u8>,
    cost: Balance,
    sender: AccountId
}

/// Demand message.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Demand<Balance, AccountId> {
    model: Vec<u8>,
    objective: Vec<u8>,
    cost: Balance,
    sender: AccountId
}

/// Liability params.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Liability<Balance, AccountId> {
    model: Vec<u8>,
    objective: Vec<u8>,
    cost: Balance,
    promisee: AccountId,
    promisor: AccountId,
    result: Option<Vec<u8>>
}

pub trait Trait: balances::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        /// Send demand and create liability when matched.
        pub fn demand(origin, model: Vec<u8>, objective: Vec<u8>, cost: T::Balance) -> Result {
            // Ensure we have a signed message, and derive the sender's account id from the signature
            let sender = ensure_signed(origin)?;

            let lot = (model.clone(), objective.clone(), cost);
            let demand_hash = T::Hashing::hash_of(&lot);
            let demand = Demand {
                model: model.into(),
                objective: objective.into(),
                cost,
                sender
            };

            Self::deposit_event(RawEvent::NewDemand(demand_hash.clone(), demand.clone()));

            match Self::offer_of(demand_hash) {
                None => <DemandOf<T>>::insert(demand_hash, demand),
                Some(offer) => {
                    Self::create_liability(demand, offer);
                    <OfferOf<T>>::remove(demand_hash)
                },
            }

            Ok(())
        }
        
        /// Send offer and create liability when matched.
        pub fn offer(origin, model: Vec<u8>, objective: Vec<u8>, cost: T::Balance) -> Result {
            // Ensure we have a signed message, and derive the sender's account id from the signature
            let sender = ensure_signed(origin)?;

            let lot = (model.clone(), objective.clone(), cost);
            let offer_hash = T::Hashing::hash_of(&lot);
            let offer = Offer {
                model: model.into(),
                objective: objective.into(),
                cost,
                sender
            };

            Self::deposit_event(RawEvent::NewOffer(offer_hash.clone(), offer.clone()));

            match Self::demand_of(offer_hash) {
                None => <OfferOf<T>>::insert(offer_hash, offer),
                Some(demand) => {
                    Self::create_liability(demand, offer);
                    <DemandOf<T>>::remove(offer_hash)
                },
            }

            Ok(())
        }

        /// Send result to finalize liability.
        pub fn finalize(origin, liability_id: u64, result: Vec<u8>) -> Result {
            // Ensure we have a signed message, and derive the sender's account id from the signature
            let sender = ensure_signed(origin)?;

            match Self::liability_of(liability_id) {
                None => Err("liability not found"),
                Some(liability) =>
                    if sender == liability.promisor {
                        <LiabilityOf<T>>::insert(liability_id, Liability { result: Some(result.clone()), .. liability });
                        Self::deposit_event(RawEvent::Finalized(liability_id, result));
                        Ok(())
                    } else {
                        Err("this is call for promisor only")
                    }
            }
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Robonomics {
        /// Get demand by hash.
        pub DemandOf get(demand_of):
            map T::Hash => Option<Demand<T::Balance, T::AccountId>>;

        /// Get offer by hash.
        pub OfferOf get(offer_of):
            map T::Hash => Option<Offer<T::Balance, T::AccountId>>;

        /// Count of created liabilities.
        pub LiabilityCount get(liability_count) config(): u64 = 0;

        /// Get liability by id.
        pub LiabilityOf get(liability_of):
            map u64 => Option<Liability<T::Balance, T::AccountId>>;

    }
}

/// Event for this module.
decl_event! {
    pub enum Event<T>
        where <T as system::Trait>::Hash,
              <T as system::Trait>::AccountId,
              <T as balances::Trait>::Balance
    {
        /// A Demand was published.
        NewDemand(Hash, Demand<Balance, AccountId>),

        /// An Offer was published.
        NewOffer(Hash, Offer<Balance, AccountId>),

        NewLiability(u64, Liability<Balance, AccountId>),

        Finalized(u64, Vec<u8>),
    }
}

impl<T: Trait> Module<T> {
    fn create_liability(demand: Demand<T::Balance,T::AccountId>, offer: Offer<T::Balance,T::AccountId>) {
        let Demand { model, objective, cost, sender: promisee } = demand;
        let Offer { model: _m, objective: _o, cost: _c, sender: promisor } = offer;

        let id = Self::liability_count();
        let liability = Liability { model, objective, cost, promisee, promisor, result: None };
        Self::deposit_event(RawEvent::NewLiability(id, liability.clone()));

        <LiabilityOf<T>>::insert(id, liability);
        <LiabilityCount<T>>::put(id + 1);
    }
}
