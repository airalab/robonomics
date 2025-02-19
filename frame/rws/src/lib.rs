///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2025 Robonomics Network <research@robonomics.network>
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

//#[cfg(test)]
//mod tests;

#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug, MaxEncodedLen)]
pub enum Subscription {
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

impl Default for Subscription {
    fn default() -> Self {
        Subscription::Daily { days: 0 }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug, MaxEncodedLen)]
pub struct AuctionLedger<AccountId: MaxEncodedLen, Balance: HasCompact + MaxEncodedLen> {
    /// Auction winner address.
    pub winner: Option<AccountId>,
    /// Current best price.
    #[codec(compact)]
    pub best_price: Balance,
    /// Kind of subscription for this auction
    pub kind: Subscription,
}

impl<AccountId: MaxEncodedLen, Balance: HasCompact + MaxEncodedLen + Default>
    AuctionLedger<AccountId, Balance>
{
    pub fn new(kind: Subscription) -> Self {
        Self {
            winner: None,
            best_price: Default::default(),
            kind,
        }
    }

    pub fn empty() -> Self {
        Self {
            winner: None,
            best_price: Default::default(),
            kind: Default::default(),
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
    /// Kind of subscription (lifetime, daily, etc).
    kind: Subscription,
}

impl<Moment: HasCompact + MaxEncodedLen + Clone> SubscriptionLedger<Moment> {
    pub fn new(last_update: Moment, kind: Subscription) -> Self {
        Self {
            free_weight: Default::default(),
            issue_time: last_update.clone(),
            last_update,
            kind,
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
    use sp_runtime::{
        traits::{AtLeast32Bit, StaticLookup},
        DispatchResult,
    };
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
        type Moment: Parameter + AtLeast32Bit + Into<u64> + MaxEncodedLen;
        /// The auction index value.
        type AuctionIndex: Parameter + AtLeast32Bit + Default + MaxEncodedLen;
        /// The auction bid currency.
        type AuctionCurrency: ReservableCurrency<Self::AccountId>;
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Reference call weight, general transaction consumes this weight.
        #[pallet::constant]
        type ReferenceCallWeight: Get<u64>;
        /// Subscription auction duration in blocks.
        #[pallet::constant]
        type AuctionDuration: Get<BlockNumberFor<Self>>;
        /// How much token should be bonded to launch new auction.
        #[pallet::constant]
        type AuctionCost: Get<BalanceOf<Self>>;
        /// Minimal auction bid.
        #[pallet::constant]
        type MinimalBid: Get<BalanceOf<Self>>;
        #[pallet::constant]
        type MaxDevicesAmount: Get<u32>;
        #[pallet::constant]
        type MaxAuctionIndexesAmount: Get<u32>;
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
        /// This call is for oracle only.
        OracleOnlyCall,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New subscription auction bid received.
        NewBid(T::AuctionIndex, T::AccountId, BalanceOf<T>),
        /// Runtime method executed using RWS subscription.
        NewCall(T::AccountId, DispatchResult),
        /// Registered RWS subscription devices.
        NewDevices(T::AccountId, Vec<T::AccountId>),
        /// Registered new RWS subscription.
        NewSubscription(T::AccountId, Subscription),
        /// Started new RWS subscription auction.
        NewAuction(Subscription, T::AuctionIndex),
    }

    #[pallet::storage]
    #[pallet::getter(fn oracle)]
    /// The `AccountId` of Ethereum RWS oracle.
    pub(super) type Oracle<T: Config> = StorageValue<_, T::AccountId>;

    #[pallet::storage]
    #[pallet::getter(fn ledger)]
    /// RWS subscriptions storage.
    pub(super) type Ledger<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, SubscriptionLedger<<T::Time as Time>::Moment>>;

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

    /// Ongoing subscription auctions.
    #[pallet::storage]
    #[pallet::getter(fn auction_queue)]
    pub(super) type AuctionQueue<T: Config> =
        StorageValue<_, BoundedVec<T::AuctionIndex, T::MaxAuctionIndexesAmount>, ValueQuery>;

    /// Next auction index.
    #[pallet::storage]
    #[pallet::getter(fn auction_next)]
    pub(super) type AuctionNext<T: Config> = StorageValue<_, T::AuctionIndex, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn auction)]
    /// Indexed auction ledger.
    pub(super) type Auction<T: Config> =
        StorageMap<_, Twox64Concat, T::AuctionIndex, AuctionLedger<T::AccountId, BalanceOf<T>>>;

    /// This is intermediate value used to escape bonded value loss.
    /// For each bond if value is not enough to issue new subscription then bonded value will
    /// be written here.
    #[pallet::storage]
    #[pallet::getter(fn unspend_bond_value)]
    pub(super) type UnspendBondValue<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            if now % T::AuctionDuration::get() == 0u32.into() {
                Self::rotate_auctions()
            } else {
                Default::default()
            }
        }
    }

    #[pallet::call]
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
        pub fn call(
            origin: OriginFor<T>,
            subscription_id: T::AccountId,
            call: Box<<T as Config>::Call>,
        ) -> DispatchResultWithPostInfo {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;

            let devices = Self::devices(&subscription_id);
            ensure!(
                devices.iter().any(|i| *i == sender),
                Error::<T>::NotLinkedDevice,
            );

            let call_info = call.get_dispatch_info();
            Self::update_subscription(&subscription_id, call_info.weight)?;

            let res =
                call.dispatch_bypass_filter(frame_system::RawOrigin::Signed(sender.clone()).into());

            Self::deposit_event(Event::NewCall(sender, res.map(|_| ()).map_err(|e| e.error)));
            res
        }

        /// Plasce a bid for live subscription auction.
        ///
        /// # <weight>
        /// - reads auction & auction_queue
        /// - writes auction bid
        /// - AuctionCurrency reserve & unreserve
        /// # </weight>
        #[pallet::weight(100_000)]
        pub fn bid(
            origin: OriginFor<T>,
            index: T::AuctionIndex,
            #[pallet::compact] amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;

            let queue = Self::auction_queue();
            ensure!(
                queue.iter().any(|i| *i == index),
                Error::<T>::NotLiveAuction,
            );

            let mut auction = Self::auction(&index).ok_or(Error::<T>::NotExistAuction)?;
            if let Some(winner) = &auction.winner {
                ensure!(auction.best_price < amount, Error::<T>::TooSmallBid);

                T::AuctionCurrency::reserve(&sender, amount.clone())?;
                T::AuctionCurrency::unreserve(&winner, auction.best_price);
                auction.winner = Some(sender.clone());
                auction.best_price = amount.clone();
            } else {
                ensure!(T::MinimalBid::get() < amount, Error::<T>::TooSmallBid);

                T::AuctionCurrency::reserve(&sender, amount.clone())?;
                auction.winner = Some(sender.clone());
                auction.best_price = amount.clone();
            }
            <Auction<T>>::insert(&index, auction);

            Self::deposit_event(Event::NewBid(index, sender, amount));
            Ok(().into())
        }

        /// Set RWS subscription devices.
        ///
        /// # <weight>
        /// - O(1).
        /// - Limited storage reads.
        /// - One DB change.
        /// # </weight>
        #[pallet::weight(100_000)]
        pub fn set_devices(
            origin: OriginFor<T>,
            devices: BoundedVec<T::AccountId, T::MaxDevicesAmount>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            <Devices<T>>::insert(sender.clone(), devices.clone());
            Self::deposit_event(Event::NewDevices(sender, devices.to_vec()));
            Ok(().into())
        }

        /// Change account bandwidth share rate by authority.
        ///
        /// Change RWS oracle account.
        ///
        /// The dispatch origin for this call must be _Root_.
        ///
        /// # <weight>
        /// - O(1).
        /// - Limited storage reads.
        /// - One DB change.
        /// # </weight>
        #[pallet::weight(100_000)]
        pub fn set_oracle(
            origin: OriginFor<T>,
            new: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <Oracle<T>>::put(T::Lookup::lookup(new)?);
            Ok(().into())
        }

        /// Change account bandwidth share rate by authority.
        ///
        /// The dispatch origin for this call must be _oracle_.
        ///
        /// # <weight>
        /// - O(1).
        /// - Limited storage reads.
        /// - One DB change.
        /// # </weight>
        #[pallet::weight(100_000)]
        pub fn set_subscription(
            origin: OriginFor<T>,
            target: T::AccountId,
            subscription: Subscription,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(
                Some(sender) == <Oracle<T>>::get(),
                Error::<T>::OracleOnlyCall
            );
            <Ledger<T>>::insert(
                target.clone(),
                SubscriptionLedger::new(T::Time::now(), subscription.clone()),
            );
            Self::deposit_event(Event::NewSubscription(target, subscription));
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
        #[pallet::weight(100_000)]
        pub fn start_auction(
            origin: OriginFor<T>,
            kind: Subscription,
        ) -> DispatchResultWithPostInfo {
            let _ = ensure_root(origin)?;
            Self::new_auction(kind);
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Create new auction.
        fn new_auction(kind: Subscription) {
            // get next index and increment
            let index = Self::auction_next();
            <AuctionNext<T>>::mutate(|x| *x += 1u8.into());

            // insert auction ledger
            <Auction<T>>::insert(&index, AuctionLedger::new(kind.clone()));

            // insert auction into queue
            let _ = <AuctionQueue<T>>::mutate(|queue| queue.try_push(index.clone()));

            // deposit descriptive event
            Self::deposit_event(Event::NewAuction(kind, index));
        }

        /// Rotate current auctions, register subscriptions and queue next.
        fn rotate_auctions() -> Weight {
            let queue = Self::auction_queue();
            let (finished, next): (Vec<_>, Vec<_>) = queue
                .iter()
                .map(|index| {
                    (
                        index.clone(),
                        Self::auction(index).unwrap_or(AuctionLedger::empty()),
                    )
                })
                .partition(|(_, auction)| auction.winner.is_some());

            // store auction indexes without bids to queue
            let mut indexes_without_bids = BoundedVec::new();
            let _ = next
                .iter()
                .map(|(i, _)| indexes_without_bids.try_push(i.clone()));
            <AuctionQueue<T>>::put(indexes_without_bids);

            for (_, auction) in finished.iter() {
                if let Some(subscription_id) = &auction.winner {
                    // transfer reserve to reward pool
                    let (slash, _) =
                        T::AuctionCurrency::slash_reserved(&subscription_id, auction.best_price);
                    let _ = T::AuctionCurrency::burn(slash.peek());
                    // register subscription
                    <Ledger<T>>::insert(
                        subscription_id,
                        SubscriptionLedger::new(T::Time::now(), auction.kind.clone()),
                    );
                }
            }

            let db = T::DbWeight::get();
            db.reads(1 + queue.len() as u64) + db.writes(1 + 2 * finished.len() as u64)
        }
        /// Update subscription internals and return updated ledger.
        fn update_subscription(
            subscription_id: &T::AccountId,
            call_weight: Weight,
        ) -> Result<(), Error<T>> {
            let mut subscription =
                Self::ledger(subscription_id).ok_or(Error::<T>::NoSubscription)?;

            let now = T::Time::now();
            let utps = match subscription.kind {
                Subscription::Lifetime { tps } => tps,
                Subscription::Daily { days } => {
                    let duration_ms = <T::Time as Time>::Moment::from(days * DAYS_TO_MS);
                    // If subscription active then 0.01 TPS else 0 TPS
                    if now < subscription.issue_time.clone() + duration_ms {
                        10_000 // uTPS
                    } else {
                        0u32
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
                <Ledger<T>>::insert(subscription_id, subscription.clone());
                Err(Error::<T>::FreeWeightIsNotEnough)
            } else {
                subscription.free_weight -= call_weight.ref_time();
                <Ledger<T>>::insert(subscription_id, subscription.clone());
                Ok(())
            }
        }
    }
}
