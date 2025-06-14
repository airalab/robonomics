///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2024 Robonomics Network <research@robonomics.network>
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
//! Robonomics Web Services runtime module.

// This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode, HasCompact, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

pub use pallet::*;

/// Pallet weights.
pub mod weights;
pub use weights::WeightInfo;

//#[cfg(test)]
//mod tests;

/// RWS subscription modes: daily, lifetime.
#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug, MaxEncodedLen)]
pub enum SubscriptionMode {
    /// Lifetime subscription.
    Lifetime {
        /// How much Transactions Per Second this subscription gives (in uTPS).
        #[codec(compact)]
        tps: u32,
    },
    /// Daily subscription (each daily subscription have 1 TPS).
    Daily {
        /// How long days this subscription active.
        #[codec(compact)]
        days: u32,
    },
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug, MaxEncodedLen)]
pub struct AuctionLedger<AccountId, Balance, Moment>
where
    AccountId: MaxEncodedLen,
    Balance: HasCompact + MaxEncodedLen,
    Moment: HasCompact + MaxEncodedLen,
{
    /// Auction winner address.
    pub winner: Option<AccountId>,
    /// Current best price.
    #[codec(compact)]
    pub best_price: Balance,
    /// Auction creation timestamp.
    #[codec(compact)]
    pub created: Moment,
    /// Subscription mode for this auction
    pub mode: SubscriptionMode,
    /// Subscription id when claimed.
    pub subscription_id: Option<u32>,
}

impl<AccountId, Balance, Moment> AuctionLedger<AccountId, Balance, Moment>
where
    AccountId: MaxEncodedLen,
    Balance: HasCompact + MaxEncodedLen + Default,
    Moment: HasCompact + MaxEncodedLen,
{
    pub fn new(mode: SubscriptionMode, created: Moment) -> Self {
        Self {
            winner: None,
            subscription_id: None,
            best_price: Default::default(),
            mode,
            created,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug, MaxEncodedLen)]
pub struct SubscriptionLedger<Moment: HasCompact + MaxEncodedLen> {
    /// Free execution weights accumulator.
    #[codec(compact)]
    free_weight: u64,
    /// Subscription creation timestamp.
    #[codec(compact)]
    issue_time: Moment,
    /// Moment of time for last subscription update (used for TPS estimation).
    #[codec(compact)]
    last_update: Moment,
    /// Subscription mode (lifetime, daily, etc).
    mode: SubscriptionMode,
}

impl<Moment: HasCompact + MaxEncodedLen + Clone> SubscriptionLedger<Moment> {
    pub fn new(last_update: Moment, mode: SubscriptionMode) -> Self {
        Self {
            free_weight: Default::default(),
            issue_time: last_update.clone(),
            last_update,
            mode,
        }
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        dispatch::GetDispatchInfo,
        pallet_prelude::*,
        traits::{Currency, Imbalance, ReservableCurrency, Time, UnfilteredDispatchable},
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{traits::AtLeast32Bit, DispatchResult};
    use sp_std::prelude::*;

    type BalanceOf<T> = <<T as Config>::AuctionCurrency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    const DAYS_TO_MS: u32 = 24 * 60 * 60 * 1000;
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Call subscription method.
        type Call: Parameter
            + UnfilteredDispatchable<RuntimeOrigin = Self::RuntimeOrigin>
            + GetDispatchInfo;
        /// Current time source.
        type Time: Time<Moment = Self::Moment>;
        /// Time should be aligned to weights for TPS calculations.
        type Moment: Parameter + AtLeast32Bit + Into<u64> + HasCompact + MaxEncodedLen;
        /// The auction bid currency.
        type AuctionCurrency: ReservableCurrency<Self::AccountId>;
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Call weights.
        type WeightInfo: weights::WeightInfo;
        /// Reference call weight, general transaction consumes this weight.
        #[pallet::constant]
        type ReferenceCallWeight: Get<u64>;
        /// Subscription auction duration in blocks.
        #[pallet::constant]
        type AuctionDuration: Get<Self::Moment>;
        /// Minimal auction bid.
        #[pallet::constant]
        type MinimalBid: Get<BalanceOf<Self>>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Auction with the index doesn't exist.
        NotExistAuction,
        /// The bid is too small.
        TooSmallBid,
        /// Subscription is not registered.
        NoSubscription,
        /// The origin account have no enough free weight to process these call: [free_weight, required_weight].
        FreeWeightIsNotEnough,
        /// Subscription time is over
        SubscriptionIsOver,
        /// Auction bidding period is over and auction already have winner.
        BiddingPeriodIsOver,
        /// Auction claim is not allowed for this user (not winner or auction isn't finish).
        ClaimIsNotAllowed,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New subscription auction bid received.
        NewBid(u32, T::AccountId, BalanceOf<T>),
        /// Runtime method executed using RWS subscription.
        RwsCall(T::AccountId, u32, DispatchResult),
        /// Subscription auction has been started.
        AuctionStarted(u32),
        /// Subscription auction finished.
        AuctionFinished(u32),
        /// RWS subscription activated for `AccountId`.
        SubscriptionActivated(T::AccountId, u32),
    }

    #[pallet::storage]
    #[pallet::getter(fn subscription)]
    /// Subscriptions stored as double map: owner account and subscription id.
    pub(super) type Subscription<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Twox64Concat,
        u32,
        SubscriptionLedger<<T::Time as Time>::Moment>,
    >;

    #[pallet::storage]
    #[pallet::getter(fn auction)]
    /// List of all auctions.
    pub(super) type Auction<T: Config> = CountedStorageMap<
        _,
        Twox64Concat,
        u32,
        AuctionLedger<T::AccountId, BalanceOf<T>, <T::Time as Time>::Moment>,
    >;

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call(weight(<T as Config>::WeightInfo))]
    impl<T: Config> Pallet<T> {
        /// Authenticates the RWS staker and dispatches a free function call.
        ///
        /// The dispatch origin for this call must be _Signed_.
        ///
        /// # <weight>
        /// - Dependes of call method.
        /// - Basically this sould be free by concept.
        /// # </weight>
        #[pallet::weight((0, call.get_dispatch_info().class, Pays::No))]
        #[pallet::call_index(0)]
        pub fn call(
            origin: OriginFor<T>,
            subscription_id: u32,
            call: Box<<T as Config>::Call>,
        ) -> DispatchResultWithPostInfo {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;

            // Ensure that subscription owner or any of subscription devices call this method
            let mut subscription = <Subscription<T>>::get(&sender, &subscription_id)
                .ok_or(Error::<T>::NoSubscription)?;

            let now = T::Time::now();
            let utps = match subscription.mode {
                SubscriptionMode::Lifetime { tps } => tps,
                SubscriptionMode::Daily { days } => {
                    let duration_ms = <T::Time as Time>::Moment::from(days * DAYS_TO_MS);
                    // If subscription active then 0.01 TPS else throw an error
                    if now < subscription.issue_time.clone() + duration_ms {
                        10_000 // uTPS
                    } else {
                        Err(Error::<T>::SubscriptionIsOver)?
                    }
                }
            };

            // Reference call weight * TPS * secons passed from last update
            let delta: u64 = (now.clone() - subscription.last_update).into();
            subscription.last_update = now;
            subscription.free_weight +=
                T::ReferenceCallWeight::get() * (utps as u64) * delta / 1_000_000_000;

            let call_weight = call.get_dispatch_info().call_weight;
            // Ensure than free weight is enough for call
            if subscription.free_weight < call_weight.ref_time() {
                <Subscription<T>>::set(&sender, &subscription_id, Some(subscription));
                Err(Error::<T>::FreeWeightIsNotEnough)?
            } else {
                subscription.free_weight -= call_weight.ref_time();
                <Subscription<T>>::set(&sender, &subscription_id, Some(subscription));
            }

            let res =
                call.dispatch_bypass_filter(frame_system::RawOrigin::Signed(sender.clone()).into());

            Self::deposit_event(Event::RwsCall(
                sender,
                subscription_id,
                res.map(|_| ()).map_err(|e| e.error),
            ));
            res
        }

        /// Plasce a bid for live subscription auction.
        ///
        /// # <weight>
        /// - reads auction & auction_queue
        /// - writes auction bid
        /// - AuctionCurrency reserve & unreserve
        /// # </weight>
        #[pallet::call_index(1)]
        pub fn bid(
            origin: OriginFor<T>,
            auction_id: u32,
            #[pallet::compact] amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;

            let now = T::Time::now();
            let mut auction = <Auction<T>>::get(&auction_id).ok_or(Error::<T>::NotExistAuction)?;

            if let Some(winner) = &auction.winner {
                // Ensure best prices is less than proposed bid
                ensure!(auction.best_price < amount, Error::<T>::TooSmallBid);
                // Ensure auction is still in bidding period
                ensure!(
                    auction.created.clone() + T::AuctionDuration::get() < now,
                    Error::<T>::BiddingPeriodIsOver,
                );

                T::AuctionCurrency::reserve(&sender, amount.clone())?;
                T::AuctionCurrency::unreserve(&winner, auction.best_price);
                auction.winner = Some(sender.clone());
                auction.best_price = amount.clone();
            } else {
                ensure!(T::MinimalBid::get() < amount, Error::<T>::TooSmallBid);

                // In case no one bid for this auction bid becomes winner
                // It's also suits for auctions out of bidding period
                T::AuctionCurrency::reserve(&sender, amount.clone())?;
                auction.winner = Some(sender.clone());
                auction.best_price = amount.clone();
            }
            <Auction<T>>::set(&auction_id, Some(auction));

            Self::deposit_event(Event::NewBid(auction_id, sender, amount));
            Ok(().into())
        }

        /// Claim a bid if win and issue new subscription.
        ///
        /// # <weight>
        /// - reads auction & auction_queue
        /// - writes auction bid
        /// - AuctionCurrency reserve & unreserve
        /// # </weight>
        #[pallet::call_index(2)]
        pub fn claim(
            origin: OriginFor<T>,
            auction_id: u32,
            beneficiary: Option<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;

            let now = T::Time::now();
            let mut auction = <Auction<T>>::get(&auction_id).ok_or(Error::<T>::NotExistAuction)?;

            // Check auction already claimed.
            ensure!(
                auction.subscription_id == None,
                Error::<T>::ClaimIsNotAllowed,
            );

            // Check auction have a winner and bidding is over.
            ensure!(
                auction.winner == Some(sender.clone()),
                Error::<T>::ClaimIsNotAllowed,
            );
            ensure!(
                auction.created.clone() + T::AuctionDuration::get() >= now,
                Error::<T>::ClaimIsNotAllowed,
            );

            // Set subscription owner to auction winner or dedicated account if set.
            let beneficiary = beneficiary.unwrap_or(sender.clone());

            // transfer reserve to reward pool
            let (slash, _) = T::AuctionCurrency::slash_reserved(&sender, auction.best_price);
            let _ = T::AuctionCurrency::burn(slash.peek());

            let subscription_id = <Subscription<T>>::iter_key_prefix(&beneficiary).count() as u32;

            // register subscription
            <Subscription<T>>::set(
                &beneficiary,
                &subscription_id,
                Some(SubscriptionLedger::new(now, auction.mode.clone())),
            );

            // Update subscription id in auction ledger
            auction.subscription_id = Some(subscription_id);
            <Auction<T>>::set(&auction_id, Some(auction));

            Self::deposit_event(Event::AuctionFinished(auction_id));
            Self::deposit_event(Event::SubscriptionActivated(beneficiary, subscription_id));
            Ok(().into())
        }

        /// Start subscription auction.
        ///
        /// The dispatch origin for this call must be _root_.
        ///
        /// # <weight>
        /// - O(1).
        /// - Limited storage reads.
        /// - One DB change.
        /// # </weight>
        #[pallet::call_index(4)]
        pub fn start_auction(
            origin: OriginFor<T>,
            mode: SubscriptionMode,
        ) -> DispatchResultWithPostInfo {
            let _ = ensure_root(origin)?;

            let id = <Auction<T>>::count();
            <Auction<T>>::set(id, Some(AuctionLedger::new(mode, T::Time::now())));

            Self::deposit_event(Event::AuctionStarted(id));
            Ok(().into())
        }
    }
}
