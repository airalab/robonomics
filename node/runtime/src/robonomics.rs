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
//! The Robonomics substrate module. This can be compiled with `#[no_std]`, ready for Wasm.

use rstd::vec::Vec;
#[cfg(feature = "std")]
use serde_derive::{Serialize, Deserialize};
use codec::{Encode, Decode};
use system::ensure_signed;
use support::{
    StorageValue, StorageMap,
    decl_module, decl_storage, decl_event, ensure,
    traits::{ReservableCurrency, Currency}, dispatch::Result
};
use sr_primitives::traits::Hash;

/// Order params.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Order<Balance,AccountId> {
    pub model: Vec<u8>,
    pub objective: Vec<u8>,
    pub cost: Balance,
    pub custodian: Option<AccountId>,
}

/// Offer message.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Offer<Balance,AccountId> {
    pub order: Order<Balance,AccountId>,
    pub sender: AccountId,
}

/// Demand message.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Demand<Balance,AccountId> {
    pub order: Order<Balance,AccountId>,
    pub sender: AccountId, 
}

/// Liability descriptive parameters.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Liability<Balance,AccountId> {
    pub order: Order<Balance,AccountId>,
    pub promisee: AccountId,
    pub promisor: AccountId,

    pub result: Option<Vec<u8>>
}

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

/// Type used for storing an liability's index; implies the maximum number of liabilities
/// the system can hold.
type LiabilityIndex = u64;

pub trait Trait: system::Trait {
    /// Payment currency; implies the processing token for liability contract.
	type Currency: ReservableCurrency<Self::AccountId>;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin
    {
        fn deposit_event() = default;

        /// Send demand and create liability when matched.
        pub fn demand(
            origin,
            model: Vec<u8>,
            objective: Vec<u8>,
            #[compact] cost: BalanceOf<T>,
            custodian: Option<T::AccountId>
        ) -> Result {
            // Ensure we have a signed message, and derive the sender's account id from the signature
            let sender = ensure_signed(origin)?;
            let order = Order { model, objective, cost, custodian }; 
            let order_hash = T::Hashing::hash_of(&order);
            let demand = Demand { order, sender };

            if let Some(offer) = <OfferOf<T>>::mutate(order_hash, |v| v.pop()) {
                Self::create_liability(demand, offer)
            } else {
                Self::deposit_event(RawEvent::NewDemand(order_hash.clone(), demand.clone()));
                <DemandOf<T>>::mutate(order_hash, |v| v.push(demand));
                Ok(())
            }
        }
        
        /// Send offer and create liability when matched.
        pub fn offer(
            origin,
            model: Vec<u8>,
            objective: Vec<u8>,
            #[compact] cost: BalanceOf<T>,
            custodian: Option<T::AccountId>
        ) -> Result {
            // Ensure we have a signed message, and derive the sender's account id from the signature
            let sender = ensure_signed(origin)?;
            let order = Order { model, objective, cost, custodian }; 
            let order_hash = T::Hashing::hash_of(&order);
            let offer = Offer { order, sender };

            if let Some(demand) = <DemandOf<T>>::mutate(order_hash, |v| v.pop()) {
                Self::create_liability(demand, offer)
            } else {
                Self::deposit_event(RawEvent::NewOffer(order_hash.clone(), offer.clone()));
                <OfferOf<T>>::mutate(order_hash, |v| v.push(offer));
                Ok(())
            }
        }

        /// Send result to finalize liability.
        pub fn finalize(
            origin,
            liability_index: LiabilityIndex,
            result: Vec<u8>
        ) -> Result {
            // Ensure we have a signed message, and derive the sender's account id from the signature
            let sender = ensure_signed(origin)?;
            let liability = <LiabilityOf<T>>::get(liability_index).ok_or("liability not found")?;

            // Safety checks
            ensure!(None == liability.result, "liability already finalized");
            if let Some(custodian) = liability.clone().order.custodian {
                ensure!(sender == custodian, "this call is for custodian only");
            }

            // Release costs
		    T::Currency::repatriate_reserved(&liability.promisee, &liability.promisor, liability.order.cost)?;

            // Update storage
            <LiabilityOf<T>>::insert(liability_index, Liability { result: Some(result.clone()), .. liability });
            Self::deposit_event(RawEvent::Finalized(liability_index, result));

            Ok(())
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Robonomics {
        /// Get demand by hash.
        pub DemandOf get(demand_of):
            map T::Hash => Vec<Demand<BalanceOf<T>,T::AccountId>>;

        /// Get offer by hash.
        pub OfferOf get(offer_of):
            map T::Hash => Vec<Offer<BalanceOf<T>,T::AccountId>>;

        pub LiabilityCount get(liability_count): LiabilityIndex;

        /// Get liability by index.
        pub LiabilityOf get(liability_of):
            map LiabilityIndex => Option<Liability<BalanceOf<T>,T::AccountId>>;
    }
}

decl_event! {
    pub enum Event<T>
        where <T as system::Trait>::Hash,
              <T as system::Trait>::AccountId,
              Balance = BalanceOf<T>
    {
        /// Someone wants a service.
        NewDemand(Hash, Demand<Balance, AccountId>),

        /// Someone provide a service.
        NewOffer(Hash, Offer<Balance, AccountId>),

        /// Yay! New liability created.
        NewLiability(LiabilityIndex, Liability<Balance, AccountId>),

        /// Result published.
        Finalized(LiabilityIndex, Vec<u8>),
    }
}

impl<T: Trait> Module<T> {
    fn create_liability(
        demand: Demand<BalanceOf<T>,T::AccountId>,
        offer: Offer<BalanceOf<T>,T::AccountId>
    ) -> Result {
        let Demand { order, sender: promisee } = demand;
        let Offer { order: _o, sender: promisor } = offer;
        let index = Self::liability_count();

		T::Currency::reserve(&promisee, order.cost)
            .map_err(|_| "promisee's balance too low")?;

        let liability = Liability { order, promisee, promisor, result: None };
        Self::deposit_event(RawEvent::NewLiability(index, liability.clone()));
        <LiabilityOf<T>>::insert(index, liability);
        LiabilityCount::mutate(|v| *v += 1);

        Ok(())
    }
}
