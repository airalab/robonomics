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
//! # Robonomics Web Services (RWS) Pallet
//!
//! The RWS pallet provides a subscription-based fee mechanism for the Robonomics Network.
//! It allows users to acquire subscriptions through an auction system and then use those
//! subscriptions to make free (feeless) transactions up to their allocated capacity.
//!
//! ## Overview
//!
//! The RWS pallet implements a subscription model where users can:
//! - Participate in auctions to acquire subscriptions
//! - Use their subscriptions to execute transactions without paying per-transaction fees
//! - Choose between different subscription types based on their needs
//!
//! This system is particularly useful for IoT devices and automated systems that need to
//! make many transactions without managing individual transaction fees.
//!
//! ## Subscription Types
//!
//! The pallet supports two types of subscriptions:
//!
//! ### Lifetime Subscription
//!
//! A lifetime subscription with custom TPS (Transactions Per Second) allocation that never expires.
//!
//! ```ignore
//! SubscriptionMode::Lifetime { tps: 10_000 } // 0.01 TPS (10,000 microTPS)
//! ```
//!
//! - **TPS**: Specified in microTPS (μTPS), where 1 TPS = 1,000,000 μTPS
//! - **Duration**: Never expires
//! - **Use case**: Long-term users who need consistent transaction capacity
//!
//! ### Daily Subscription
//!
//! A time-limited subscription with a fixed TPS allocation of 0.01 TPS.
//!
//! ```ignore
//! SubscriptionMode::Daily { days: 30 } // Fixed 0.01 TPS for 30 days
//! ```
//!
//! - **TPS**: Fixed at 10,000 μTPS (0.01 TPS)
//! - **Duration**: Expires after the specified number of days
//! - **Use case**: Short-term or trial users
//!
//! ## Auction Lifecycle
//!
//! The subscription acquisition process follows a multi-phase auction lifecycle:
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                    AUCTION LIFECYCLE                             │
//! └──────────────────────────────────────────────────────────────────┘
//!
//! 1. CREATION
//!    ┌─────────────────┐
//!    │ start_auction() │ ← Root authority creates auction
//!    │  (Root only)    │
//!    └────────┬────────┘
//!             │
//!             v
//! 2. BIDDING PERIOD
//!    ┌─────────────────┐
//!    │   bid()         │ ← First bid starts timer
//!    │   (Anyone)      │    (AuctionDuration countdown begins)
//!    └────────┬────────┘
//!             │         ← More bids can be placed
//!             │           (Must be higher than current best)
//!             │
//!             v
//!    ┌─────────────────┐
//!    │  Wait for       │
//!    │ AuctionDuration │ ← Fixed time period after first bid
//!    └────────┬────────┘
//!             │
//!             v
//! 3. CLAIM PHASE
//!    ┌─────────────────┐
//!    │   claim()       │ ← Winner claims after AuctionDuration
//!    │  (Winner only)  │    (Creates subscription)
//!    └────────┬────────┘
//!             │
//!             v
//! 4. USAGE
//!    ┌─────────────────┐
//!    │   call()        │ ← Owner uses subscription for feeless calls
//!    │ (Owner only)    │
//!    └─────────────────┘
//! ```
//!
//! ### Auction Rules
//!
//! - **Creation**: Only root authority can start auctions via `start_auction()`
//! - **First Bid**: Must exceed `MinimalBid` and starts the `AuctionDuration` timer
//! - **Subsequent Bids**: Must exceed current `best_price`, previous bidder's funds are unreserved
//! - **Bidding Period**: Lasts for `AuctionDuration` milliseconds after the first bid
//! - **Claiming**: Only the winner can claim after `AuctionDuration` has passed
//! - **Payment**: Winner's bid is slashed and burned upon claiming
//!
//! ## Free Weight Mechanism
//!
//! Subscriptions use a "free weight" system to track transaction capacity:
//!
//! ### Weight Accumulation Formula
//!
//! ```text
//! free_weight += ReferenceCallWeight × μTPS × Δt_seconds / 1_000_000_000
//! ```
//!
//! Where:
//! - `ReferenceCallWeight`: Base weight cost for a standard transaction
//! - `μTPS`: Subscription's TPS in microTPS (1 TPS = 1,000,000 μTPS)
//! - `Δt_seconds`: Time elapsed since last update in seconds
//! - `1_000_000_000`: Conversion factor from seconds to nanoseconds
//!
//! ### Weight Consumption
//!
//! When executing a call:
//! 1. Free weight is accumulated based on time elapsed
//! 2. Call's weight requirement is checked against available free weight
//! 3. If sufficient, the weight is deducted and call executes feeless
//! 4. If insufficient, the call is rejected with `FreeWeightIsNotEnough` error
//!
//! ### TPS Calculation Example
//!
//! For a subscription with 10,000 μTPS (0.01 TPS):
//! - Over 1 second: accumulates `ReferenceCallWeight × 10,000 / 1,000,000,000` weight units
//! - Over 100 seconds: can execute approximately 1 reference transaction
//!
//! ## Storage Structure
//!
//! The pallet uses the following storage items:
//!
//! ### Auction Storage
//!
//! ```ignore
//! type Auction = CountedStorageMap<auction_id, AuctionLedger>
//! ```
//!
//! A counted map storing all auctions by their ID. The counter automatically increments
//! when new auctions are created via `start_auction()`.
//!
//! **AuctionLedger** contains:
//! - `winner`: Current highest bidder (None if no bids yet)
//! - `best_price`: Current highest bid amount
//! - `first_bid_time`: Timestamp when first bid was placed (starts timer)
//! - `mode`: Subscription type being auctioned
//! - `subscription_id`: ID of subscription after claiming (None until claimed)
//!
//! ### Subscription Storage
//!
//! ```ignore
//! type Subscription = StorageDoubleMap<account_id, subscription_id, SubscriptionLedger>
//! ```
//!
//! A double map allowing users to own multiple subscriptions, indexed by account and subscription ID.
//!
//! **SubscriptionLedger** contains:
//! - `free_weight`: Accumulated weight available for transactions
//! - `issue_time`: When subscription was created
//! - `last_update`: Last time free_weight was updated
//! - `mode`: Subscription type (Lifetime or Daily)
//! - `expiration_time`: When Daily subscription expires (None for Lifetime)
//!
//! ## Usage Examples
//!
//! ### Starting an Auction
//!
//! ```ignore
//! // Root starts a lifetime subscription auction with 0.1 TPS
//! RWS::start_auction(
//!     RuntimeOrigin::root(),
//!     SubscriptionMode::Lifetime { tps: 100_000 }
//! )?;
//!
//! // Root starts a 30-day subscription auction
//! RWS::start_auction(
//!     RuntimeOrigin::root(),
//!     SubscriptionMode::Daily { days: 30 }
//! )?;
//! ```
//!
//! ### Bidding on an Auction
//!
//! ```ignore
//! // Alice bids 1000 tokens on auction 0
//! RWS::bid(
//!     RuntimeOrigin::signed(alice),
//!     0, // auction_id
//!     1000
//! )?;
//!
//! // Bob outbids Alice with 1500 tokens
//! RWS::bid(
//!     RuntimeOrigin::signed(bob),
//!     0,
//!     1500
//! )?;
//! ```
//!
//! ### Claiming a Won Auction
//!
//! ```ignore
//! // After AuctionDuration has passed, Bob claims the subscription
//! RWS::claim(
//!     RuntimeOrigin::signed(bob),
//!     0, // auction_id
//!     None // Bob will be the subscription owner
//! )?;
//!
//! // Or Bob can specify a different beneficiary
//! RWS::claim(
//!     RuntimeOrigin::signed(bob),
//!     0,
//!     Some(charlie) // Charlie becomes the subscription owner
//! )?;
//! ```
//!
//! ### Using a Subscription
//!
//! ```ignore
//! // Bob uses his subscription (id: 0) to execute a transfer
//! RWS::call(
//!     RuntimeOrigin::signed(bob),
//!     0, // subscription_id
//!     Box::new(RuntimeCall::Balances(
//!         pallet_balances::Call::transfer_allow_death {
//!             dest: alice,
//!             value: 100
//!         }
//!     ))
//! )?;
//! ```
//!
//! ## Errors
//!
//! - `NotExistAuction`: Auction with the given ID doesn't exist
//! - `TooSmallBid`: Bid amount is below minimum or current best price
//! - `NoSubscription`: Subscription doesn't exist for this account
//! - `FreeWeightIsNotEnough`: Insufficient accumulated weight for the call
//! - `SubscriptionIsOver`: Daily subscription has expired
//! - `BiddingPeriodIsOver`: Cannot bid after AuctionDuration has passed
//! - `ClaimIsNotAllowed`: Caller is not winner or bidding period hasn't ended

// This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, HasCompact, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migrations;
#[cfg(test)]
pub mod mock;
pub mod weights;

pub use pallet::*;
pub use weights::WeightInfo;

#[cfg(test)]
mod tests;

/// Number of milliseconds in a day.
const DAYS_TO_MS: u32 = 24 * 60 * 60 * 1000;

/// RWS subscription modes: Lifetime and Daily.
///
/// Subscriptions determine how transaction capacity is allocated to users.
/// Each mode has different characteristics regarding TPS allocation and duration.
///
/// # Examples
///
/// Creating a Lifetime subscription with custom TPS:
/// ```ignore
/// let lifetime = SubscriptionMode::Lifetime { tps: 50_000 }; // 0.05 TPS
/// ```
///
/// Creating a Daily subscription (always 0.01 TPS):
/// ```ignore
/// let daily = SubscriptionMode::Daily { days: 7 }; // 1 week, 0.01 TPS
/// ```
#[derive(
    PartialEq,
    Eq,
    Clone,
    Encode,
    Decode,
    TypeInfo,
    RuntimeDebug,
    MaxEncodedLen,
    DecodeWithMemTracking,
)]
pub enum SubscriptionMode {
    /// Lifetime subscription with custom TPS allocation that never expires.
    ///
    /// This mode allows specifying any TPS value and the subscription remains
    /// valid indefinitely. Ideal for users who need consistent, long-term
    /// transaction capacity.
    ///
    /// # Fields
    ///
    /// * `tps` - Transactions Per Second in microTPS (μTPS), where 1 TPS = 1,000,000 μTPS.
    ///   For example, 10_000 μTPS = 0.01 TPS.
    Lifetime {
        /// How much Transactions Per Second this subscription gives (in μTPS).
        ///
        /// This value determines how quickly free weight accumulates for the subscription.
        /// Higher TPS means more transactions can be executed per unit of time.
        #[codec(compact)]
        tps: u32,
    },
    /// Daily subscription with fixed 0.01 TPS that expires after a specified duration.
    ///
    /// This mode always provides 10,000 μTPS (0.01 TPS) and expires after the
    /// specified number of days. The expiration time is calculated as:
    /// `issue_time + (days × 24 × 60 × 60 × 1000)` milliseconds.
    ///
    /// # Fields
    ///
    /// * `days` - Number of days the subscription remains active.
    Daily {
        /// How many days this subscription remains active.
        ///
        /// After this period expires (calculated from issue_time), attempts to use
        /// the subscription will fail with `SubscriptionIsOver` error.
        #[codec(compact)]
        days: u32,
    },
}

/// Auction state tracking structure.
///
/// This structure maintains all information about an ongoing or completed auction.
/// It tracks the current winner, best bid, timing information, and the subscription
/// mode being auctioned.
///
/// # Lifecycle States
///
/// 1. **Initial**: `winner = None`, `first_bid_time = None` - Auction created, no bids yet
/// 2. **Bidding**: `winner = Some(account)`, `first_bid_time = Some(time)` - Active bidding
/// 3. **Claiming**: After `first_bid_time + AuctionDuration` - Winner can claim
/// 4. **Claimed**: `subscription_id = Some(id)` - Auction completed
///
/// # Examples
///
/// ```ignore
/// // Create a new auction for a Lifetime subscription
/// let auction = AuctionLedger::new(SubscriptionMode::Lifetime { tps: 100_000 });
/// assert_eq!(auction.winner, None);
/// assert_eq!(auction.best_price, 0);
/// ```
#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug, MaxEncodedLen)]
pub struct AuctionLedger<AccountId, Balance, Moment>
where
    AccountId: MaxEncodedLen,
    Balance: HasCompact + MaxEncodedLen,
    Moment: HasCompact + MaxEncodedLen,
{
    /// Current auction winner (highest bidder).
    ///
    /// - `None` if no bids have been placed yet
    /// - `Some(account)` once the first valid bid is received
    ///
    /// This account has their bid amount reserved in the AuctionCurrency.
    /// If outbid, their reserved funds are released and the new winner's funds are reserved.
    pub winner: Option<AccountId>,
    
    /// Current highest bid amount.
    ///
    /// - `0` initially (no bids placed)
    /// - Updated when a higher bid is accepted
    ///
    /// New bids must exceed this amount (for auctions with existing bids) or
    /// exceed `MinimalBid` (for first bid).
    #[codec(compact)]
    pub best_price: Balance,
    
    /// Timestamp when the first bid was placed.
    ///
    /// - `None` if no bids have been placed
    /// - `Some(timestamp)` once first bid is accepted
    ///
    /// This timestamp is crucial as it starts the `AuctionDuration` countdown.
    /// The auction can only be claimed after `first_bid_time + AuctionDuration`.
    pub first_bid_time: Option<Moment>,
    
    /// The subscription mode being auctioned.
    ///
    /// This determines what type of subscription the winner will receive:
    /// - `Lifetime { tps }` for permanent subscriptions with custom TPS
    /// - `Daily { days }` for time-limited subscriptions with fixed 0.01 TPS
    pub mode: SubscriptionMode,
    
    /// Subscription ID assigned when auction is claimed.
    ///
    /// - `None` until the winner calls `claim()`
    /// - `Some(id)` after successful claim
    ///
    /// Once set, the auction is considered complete and cannot be claimed again.
    pub subscription_id: Option<u32>,
}

impl<AccountId, Balance, Moment> AuctionLedger<AccountId, Balance, Moment>
where
    AccountId: MaxEncodedLen,
    Balance: HasCompact + MaxEncodedLen + Default,
    Moment: HasCompact + MaxEncodedLen,
{
    pub fn new(mode: SubscriptionMode) -> Self {
        Self {
            winner: None,
            subscription_id: None,
            best_price: Default::default(),
            mode,
            first_bid_time: None,
        }
    }
}

/// Subscription state and capacity tracking structure.
///
/// This structure maintains the state of an active subscription, including its
/// accumulated free weight, timing information, and subscription parameters.
///
/// # Free Weight System
///
/// The `free_weight` field accumulates over time based on the subscription's TPS:
///
/// ```text
/// free_weight += ReferenceCallWeight × μTPS × Δt_seconds / 1_000_000_000
/// ```
///
/// When a transaction is executed via `call()`, the required weight is deducted
/// from `free_weight`. If insufficient weight is available, the call is rejected.
///
/// # Expiration
///
/// - **Lifetime subscriptions**: `expiration_time = None`, never expire
/// - **Daily subscriptions**: `expiration_time = Some(issue_time + days × DAYS_TO_MS)`
///
/// # Examples
///
/// ```ignore
/// // Create a lifetime subscription
/// let lifetime_sub = SubscriptionLedger::new(
///     now,
///     SubscriptionMode::Lifetime { tps: 50_000 }
/// );
/// assert_eq!(lifetime_sub.expiration_time, None);
///
/// // Create a daily subscription
/// let daily_sub = SubscriptionLedger::new(
///     now,
///     SubscriptionMode::Daily { days: 7 }
/// );
/// assert!(daily_sub.expiration_time.is_some());
/// ```
#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug, MaxEncodedLen)]
pub struct SubscriptionLedger<Moment: HasCompact + MaxEncodedLen> {
    /// Accumulated free execution weight available for transactions.
    ///
    /// This value increases over time based on the subscription's TPS and decreases
    /// when transactions are executed. The accumulation formula is:
    ///
    /// `free_weight += ReferenceCallWeight × μTPS × elapsed_seconds / 1_000_000_000`
    ///
    /// When executing a call, the call's weight is checked against this value.
    /// If sufficient, the weight is deducted; otherwise, the call fails with
    /// `FreeWeightIsNotEnough` error.
    #[codec(compact)]
    free_weight: u64,
    
    /// Timestamp when the subscription was created.
    ///
    /// This is set when the subscription is first created via `claim()` and never changes.
    /// Used for record-keeping and calculating expiration time for Daily subscriptions.
    #[codec(compact)]
    issue_time: Moment,
    
    /// Timestamp of the last subscription update.
    ///
    /// Updated each time `call()` is invoked to use the subscription. This timestamp
    /// is used to calculate the time elapsed (Δt) for free weight accumulation.
    /// The longer the time since last update, the more free weight accumulates.
    #[codec(compact)]
    last_update: Moment,
    
    /// The subscription mode (Lifetime or Daily).
    ///
    /// Determines:
    /// - TPS allocation: Lifetime uses custom `tps`, Daily uses fixed 10,000 μTPS
    /// - Expiration: Lifetime never expires, Daily expires after specified days
    mode: SubscriptionMode,
    
    /// Expiration timestamp for Daily subscriptions.
    ///
    /// - `None` for Lifetime subscriptions (never expire)
    /// - `Some(timestamp)` for Daily subscriptions, calculated as:
    ///   `issue_time + (days × 24 × 60 × 60 × 1000)` milliseconds
    ///
    /// When present, calls via this subscription fail with `SubscriptionIsOver`
    /// error after the current time exceeds this timestamp.
    expiration_time: Option<Moment>,
}

impl<Moment> SubscriptionLedger<Moment>
where
    Moment: HasCompact + MaxEncodedLen + Clone + From<u32> + core::ops::Add<Output = Moment>,
{
    pub fn new(last_update: Moment, mode: SubscriptionMode) -> Self {
        let expiration_time = match mode {
            SubscriptionMode::Daily { days } => {
                let duration_ms = Moment::from(days * DAYS_TO_MS);
                Some(last_update.clone() + duration_ms)
            }
            SubscriptionMode::Lifetime { .. } => None,
        };

        Self {
            free_weight: Default::default(),
            issue_time: last_update.clone(),
            last_update,
            mode,
            expiration_time,
        }
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        dispatch::GetDispatchInfo,
        pallet_prelude::*,
        traits::{
            Currency, Imbalance, OnRuntimeUpgrade, ReservableCurrency, Time, UnfilteredDispatchable,
        },
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{traits::AtLeast32Bit, DispatchResult};
    use sp_std::prelude::*;

    type BalanceOf<T> = <<T as Config>::AuctionCurrency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

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
        #[allow(deprecated)]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Reference call weight, general transaction consumes this weight.
        #[pallet::constant]
        type ReferenceCallWeight: Get<u64>;
        /// Subscription auction duration in time units (`Moment`).
        /// The unit of `Moment` is typically milliseconds.
        #[pallet::constant]
        type AuctionDuration: Get<Self::Moment>;
        /// Minimal auction bid.
        #[pallet::constant]
        type MinimalBid: Get<BalanceOf<Self>>;
        /// Origin that can start auctions (root, governance, or automated system)
        type StartAuctionOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Extrinsic weights
        type WeightInfo: WeightInfo;
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

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_runtime_upgrade() -> Weight {
            migrations::v2::MigrateToV2::<T>::on_runtime_upgrade()
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
            migrations::v2::MigrateToV2::<T>::pre_upgrade()
                .map_err(|e| sp_runtime::TryRuntimeError::from(e))
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
            migrations::v2::MigrateToV2::<T>::post_upgrade(state)
                .map_err(|e| sp_runtime::TryRuntimeError::from(e))
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Authenticates the RWS staker and dispatches a free function call.
        ///
        /// The dispatch origin for this call must be _Signed_.
        ///
        /// # <weight>
        /// - Depends of call method.
        /// - Basically this should be free by concept.
        /// # </weight>
        #[pallet::call_index(0)]
        #[pallet::weight((0, call.get_dispatch_info().class, Pays::No))]
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
                SubscriptionMode::Daily { .. } => {
                    // Use cached expiration_time instead of recalculating
                    if let Some(ref expiration_time) = subscription.expiration_time {
                        // If subscription active then 0.01 TPS else throw an error
                        if now < *expiration_time {
                            10_000 // μTPS
                        } else {
                            Err(Error::<T>::SubscriptionIsOver)?
                        }
                    } else {
                        // This should never happen as Daily subscriptions always have expiration_time
                        // but handle gracefully to avoid panics
                        Err(Error::<T>::SubscriptionIsOver)?
                    }
                }
            };

            // Reference call weight * TPS * seconds passed from last update
            let delta: u64 = (now.clone() - subscription.last_update).into();
            subscription.last_update = now;
            subscription.free_weight +=
                T::ReferenceCallWeight::get()
                    .saturating_mul(utps as u64)
                    .saturating_mul(delta)
                    .saturating_div(1_000_000_000);

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
        #[pallet::weight(T::WeightInfo::bid())]
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
                // Ensure auction is still in bidding period (must have first_bid_time set)
                if let Some(ref first_bid_time) = auction.first_bid_time {
                    ensure!(
                        first_bid_time.clone() + T::AuctionDuration::get() > now,
                        Error::<T>::BiddingPeriodIsOver,
                    );
                } else {
                    // If there's a winner but no first_bid_time (should only happen with migrated auctions),
                    // reject further bids to prevent undefined behavior
                    return Err(Error::<T>::BiddingPeriodIsOver.into());
                }

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
                // Set first_bid_time when the first bid is placed
                auction.first_bid_time = Some(now);
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
        #[pallet::weight(T::WeightInfo::claim())]
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
                auction.subscription_id.is_none(),
                Error::<T>::ClaimIsNotAllowed,
            );

            // Check auction have a winner and bidding is over.
            ensure!(
                auction.winner == Some(sender.clone()),
                Error::<T>::ClaimIsNotAllowed,
            );
            // Ensure auction has received at least one bid (first_bid_time is set)
            // and the bidding period has ended
            if let Some(ref first_bid_time) = auction.first_bid_time {
                ensure!(
                    first_bid_time.clone() + T::AuctionDuration::get() <= now,
                    Error::<T>::ClaimIsNotAllowed,
                );
            } else {
                // Cannot claim auction without any bids
                return Err(Error::<T>::ClaimIsNotAllowed.into());
            }

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
        /// The dispatch origin for this call must satisfy `StartAuctionOrigin`.
        /// This allows configuration for root, governance pallets, or automated systems.
        ///
        /// # <weight>
        /// - O(1).
        /// - Limited storage reads.
        /// - One DB change.
        /// # </weight>
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::start_auction())]
        pub fn start_auction(
            origin: OriginFor<T>,
            mode: SubscriptionMode,
        ) -> DispatchResultWithPostInfo {
            T::StartAuctionOrigin::ensure_origin(origin)?;

            let id = <Auction<T>>::count();
            <Auction<T>>::set(id, Some(AuctionLedger::new(mode)));

            Self::deposit_event(Event::AuctionStarted(id));
            Ok(().into())
        }
    }
}
