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

use frame_support::pallet_prelude::Weight;
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

impl Default for SubscriptionMode {
    fn default() -> Self {
        SubscriptionMode::Daily { days: 0 }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug, MaxEncodedLen)]
pub struct AuctionLedger<AccountId, Balance, Moment, SubscriptionId>
where
    AccountId: MaxEncodedLen,
    Balance: HasCompact + MaxEncodedLen,
    Moment: HasCompact + MaxEncodedLen,
    SubscriptionId: MaxEncodedLen,
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
    /// Set when subscription issued.
    pub subscription_id: Option<SubscriptionId>,
}

impl<AccountId, Balance, Moment, SubId> AuctionLedger<AccountId, Balance, Moment, SubId>
where
    AccountId: MaxEncodedLen,
    Balance: HasCompact + MaxEncodedLen + Default,
    Moment: HasCompact + MaxEncodedLen,
    SubId: MaxEncodedLen,
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

impl<AccountId, Balance, Moment, SubId> Default for AuctionLedger<AccountId, Balance, Moment, SubId>
where
    AccountId: MaxEncodedLen,
    Balance: HasCompact + MaxEncodedLen + Default,
    Moment: HasCompact + MaxEncodedLen + Default,
    SubId: MaxEncodedLen,
{
    fn default() -> Self {
        Self {
            winner: None,
            subscription_id: None,
            best_price: Default::default(),
            created: Default::default(),
            mode: Default::default(),
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
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

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
        /// The auction index value.
        type AuctionIndex: Parameter + AtLeast32Bit + Default + MaxEncodedLen;
        /// The auction bid currency.
        type AuctionCurrency: ReservableCurrency<Self::AccountId>;
        /// Subscription identifier type, must be enumerable and have good enough capacity, eg. u64
        type SubscriptionId: Parameter + AtLeast32Bit + Default + MaxEncodedLen;
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
        #[pallet::constant]
        type MaxDevicesAmount: Get<u32>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Auction is not ongoing.
        NotLiveAuction,
        /// Auction with the index doesn't exist.
        NotExistAuction,
        /// The bid is too small.
        TooSmallBid,
        /// Subscription is not registered.
        NoSubscription,
        /// Devices isn't assigned to this subscription.
        NotLinkedDevice,
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
        NewBid(T::AuctionIndex, T::AccountId, BalanceOf<T>),
        /// Runtime method executed using RWS subscription.
        RwsCall(T::AccountId, T::SubscriptionId, DispatchResult),
        /// Subscription account update users(devices) list.
        DevicesUpdated(T::AccountId, Vec<T::AccountId>),
        /// Subscription auction has been started.
        AuctionStarted(T::AuctionIndex),
        /// Subscription auction finished.
        AuctionFinished(T::AuctionIndex),
        /// RWS subscription activated for `AccountId`.
        SubscriptionActivated(T::SubscriptionId, T::AccountId),
    }

    #[pallet::storage]
    #[pallet::getter(fn subscription)]
    /// Subscription ledger storage.
    pub(super) type Subscription<T: Config> = StorageMap<
        _,
        Twox64Concat,
        T::SubscriptionId,
        SubscriptionLedger<<T::Time as Time>::Moment>,
    >;

    #[pallet::storage]
    #[pallet::getter(fn subscription_total)]
    /// Total subscription amount (active & inactive).
    pub(super) type SubscriptionTotal<T: Config> = StorageValue<_, T::SubscriptionId, ValueQuery>;

    /// RWS subscriptions owners.
    #[pallet::storage]
    #[pallet::getter(fn subscription_owner)]
    pub(super) type SubscriptionOwner<T: Config> =
        StorageMap<_, Twox64Concat, T::SubscriptionId, T::AccountId>;

    #[pallet::storage]
    #[pallet::getter(fn devices)]
    /// Subscription linked devices.
    pub(super) type Devices<T: Config> = StorageMap<
        _,
        Twox64Concat,
        T::AccountId,
        BoundedVec<T::AccountId, T::MaxDevicesAmount>,
        ValueQuery,
    >;

    /// Total amount of actions happens.
    #[pallet::storage]
    #[pallet::getter(fn auction_total)]
    pub(super) type AuctionTotal<T: Config> = StorageValue<_, T::AuctionIndex, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn auction)]
    /// Indexed auction ledger.
    pub(super) type Auction<T: Config> = StorageMap<
        _,
        Twox64Concat,
        T::AuctionIndex,
        AuctionLedger<T::AccountId, BalanceOf<T>, <T::Time as Time>::Moment, T::SubscriptionId>,
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
            subscription_id: T::SubscriptionId,
            call: Box<<T as Config>::Call>,
        ) -> DispatchResultWithPostInfo {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;

            // Ensure that subscription owner or any of subscription devices call this method
            let owner =
                Self::subscription_owner(&subscription_id).ok_or(Error::<T>::NoSubscription)?;
            if sender != owner {
                ensure!(
                    Self::devices(&owner).iter().any(|i| *i == sender),
                    Error::<T>::NotLinkedDevice,
                );
            }

            let call_info = call.get_dispatch_info();
            Self::update_subscription(&subscription_id, call_info.call_weight)?;

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
            index: T::AuctionIndex,
            #[pallet::compact] amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;

            let now = T::Time::now();
            let mut auction = Self::auction(&index).ok_or(Error::<T>::NotExistAuction)?;
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
            <Auction<T>>::insert(&index, auction);

            Self::deposit_event(Event::NewBid(index, sender, amount));
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
            index: T::AuctionIndex,
            beneficiary: Option<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;

            let now = T::Time::now();
            let mut auction = Self::auction(&index).ok_or(Error::<T>::NotExistAuction)?;

            // Check auction have a winner and bidding is over.
            ensure!(
                auction.winner == Some(sender.clone()),
                Error::<T>::ClaimIsNotAllowed
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

            let subscription_id = Self::subscription_total();

            // register subscription
            <Subscription<T>>::insert(
                subscription_id.clone(),
                SubscriptionLedger::new(now, auction.mode.clone()),
            );

            // Set subscription owner
            <SubscriptionOwner<T>>::insert(subscription_id.clone(), beneficiary.clone());

            // Update total subscriptions amount
            <SubscriptionTotal<T>>::mutate(|x| *x += 1u8.into());

            // Update subscription id in auction ledger
            auction.subscription_id = Some(subscription_id.clone());
            <Auction<T>>::insert(&index, auction);

            Self::deposit_event(Event::SubscriptionActivated(subscription_id, beneficiary));
            Ok(().into())
        }

        /// Set RWS subscription devices.
        ///
        /// # <weight>
        /// - O(1).
        /// - Limited storage reads.
        /// - One DB change.
        /// # </weight>
        #[pallet::call_index(3)]
        pub fn set_devices(
            origin: OriginFor<T>,
            devices: BoundedVec<T::AccountId, T::MaxDevicesAmount>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            <Devices<T>>::insert(sender.clone(), devices.clone());
            Self::deposit_event(Event::DevicesUpdated(sender, devices.to_vec()));
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
            // get next index and increment
            let index = Self::auction_total();
            <AuctionTotal<T>>::mutate(|x| *x += 1u8.into());

            // insert auction ledger
            let auction: AuctionLedger<_, _, _, T::SubscriptionId> =
                AuctionLedger::new(mode, T::Time::now());
            <Auction<T>>::insert(&index, auction);

            // deposit descriptive event
            Self::deposit_event(Event::AuctionStarted(index));
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Update subscription internals and return updated ledger.
        fn update_subscription(
            subscription_id: &T::SubscriptionId,
            call_weight: Weight,
        ) -> Result<(), Error<T>> {
            let mut subscription =
                Self::subscription(subscription_id).ok_or(Error::<T>::NoSubscription)?;

            let now = T::Time::now();
            let utps = match subscription.mode {
                SubscriptionMode::Lifetime { tps } => tps,
                SubscriptionMode::Daily { days } => {
                    let duration_ms = <T::Time as Time>::Moment::from(days * DAYS_TO_MS);
                    // If subscription active then 0.01 TPS else throw an error
                    if now < subscription.issue_time.clone() + duration_ms {
                        10_000 // uTPS
                    } else {
                        return Err(Error::<T>::SubscriptionIsOver);
                    }
                }
            };

            let delta: u64 = (now.clone() - subscription.last_update).into();
            // Reference call weight * TPS * secons passed from last update
            subscription.free_weight +=
                T::ReferenceCallWeight::get() * (utps as u64) * delta / 1_000_000_000;
            subscription.last_update = now;

            // Ensure than free weight is enough for call
            if subscription.free_weight < call_weight.ref_time() {
                <Subscription<T>>::insert(subscription_id, subscription.clone());
                Err(Error::<T>::FreeWeightIsNotEnough)
            } else {
                subscription.free_weight -= call_weight.ref_time();
                <Subscription<T>>::insert(subscription_id, subscription.clone());
                Ok(())
            }
        }
    }
}
