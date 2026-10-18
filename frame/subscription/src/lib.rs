///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2026 Robonomics Network <research@robonomics.network>
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
//! # Robonomics Subscription Pallet
//!
//! The canonical runtime accounting layer for time-limited platform
//! resources. It answers one question: which resources are allocated to this
//! owner, for how long, and what is the current accounting state?
//!
//! The pallet has no extrinsics and is deliberately independent from how a
//! subscription is acquired (allocators call [`SubscriptionAllocator`]), who
//! may use another owner's resources (authorization providers), how a
//! transaction fee is paid, and what a concrete operation costs (resource
//! consumers call [`ResourceAccounting`]).
//!
//! ## Lifecycle and generations
//!
//! An owner has at most one current [`Subscription`]. Every allocation
//! replaces it and increments the owner's generation, which is never reset,
//! not even by revocation or expiry. The generation is the accounting epoch
//! of all global and local state: an operation prepared against generation
//! `N` can neither consume nor refund generation `N + 1`.
//!
//! Validity is measured with [`Config::Time`], never with block numbers:
//! `started_at = now`, `expires_at = started_at + valid_for`, and the
//! subscription is active while `now < expires_at`.
//!
//! ## Transaction resource
//!
//! The Transaction allocation is a rate in `Wn` per millisecond (`Wn` is the
//! smallest unit of XRT, `1 XRT = 10^9 Wn`) accounted with a token bucket:
//!
//! ```text
//! available = min(available + rate * elapsed_ms, rate * TransactionBurstPeriod)
//! ```
//!
//! A new generation starts with an empty bucket. The sum of all allocated
//! rates is kept in [`TotalAllocatedTransactionRate`] in O(1) and never
//! exceeds [`Config::MaxSubscriptionTransactionRate`].
//!
//! Local consumption additionally uses a per-`(owner, LocalId, resource)`
//! bucket whose rate comes from [`Config::ResourceLimits`]. Local rates are
//! not reservations and may overcommit the owner's global rate; the global
//! bucket stays authoritative. Local state of an older generation is
//! logically empty: it is reset lazily on first use of the new generation
//! (empty, accruing from the generation start like the global bucket), never
//! cleaned up eagerly, and stale entries are overwritten when reused.
//!
//! ## Expiration
//!
//! Expired subscriptions release their global rate through a bounded
//! [`ExpirationAgenda`] processed in `on_initialize`. Entries are bound to a
//! generation, so a stale entry never releases the capacity of a replacement.
//! Entries are scheduled in time buckets of [`Config::ExpirationBucketSize`]
//! rounded up, therefore capacity is released at most one bucket after the
//! expiration time (more if a bucket is full or many entries are due), while
//! consumption is rejected exactly from `expires_at`. The hook observes the
//! time of the previous block because the timestamp inherent is applied after
//! `on_initialize`.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(test)]
mod tests;
pub mod types;
pub mod weights;

pub use pallet::*;
pub use types::{
    Amount, Charge, GenerationState, Limit, RateState, ResourceState, Subscription, Usage,
};
pub use weights::WeightInfo;

/// Number of consecutive agenda buckets tried when scheduling an expiration
/// before giving up with `ExpirationAgendaFull`.
const MAX_AGENDA_PROBES: u32 = 16;

#[frame_support::pallet]
#[allow(clippy::module_inception)]
pub mod pallet {
    use super::*;
    use alloc::vec::Vec;
    use frame_support::{pallet_prelude::*, storage::with_storage_layer, traits::Time};
    use frame_system::pallet_prelude::*;
    use robonomics_subscription_primitives::{
        ChargeSource, Resource, ResourceAccounting, ResourceCharge, ResourceGeneration,
        ResourceLimit, SubscriptionAllocation, SubscriptionAllocator,
    };
    use sp_runtime::{
        traits::{CheckedAdd, One, Saturating, UniqueSaturatedInto, Zero},
        ArithmeticError,
    };

    /// Moment of the runtime clock.
    pub type MomentOf<T> = <<T as Config>::Time as Time>::Moment;

    /// Allocation payload accepted by the pallet.
    pub type AllocationOf<T> =
        SubscriptionAllocation<Resource, Limit, MomentOf<T>, <T as Config>::MaxResources>;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        #[allow(deprecated)]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Runtime clock. Moments are in milliseconds, so Transaction rates
        /// are in `Wn` per millisecond.
        type Time: Time;

        /// Opaque local accounting context (for example a CPS scope).
        type LocalId: Parameter + Member + MaxEncodedLen;

        /// Source of local limits. A missing limit forbids local
        /// consumption.
        type ResourceLimits: ResourceLimit<
            LocalId = Self::LocalId,
            ResourceId = Resource,
            Limit = Limit,
        >;

        /// Time a bucket may accumulate allowance: burst capacity is
        /// `rate * TransactionBurstPeriod`.
        #[pallet::constant]
        type TransactionBurstPeriod: Get<MomentOf<Self>>;

        /// Network capacity reserved for subscription Transaction rates, in
        /// `Wn` per millisecond.
        #[pallet::constant]
        type MaxSubscriptionTransactionRate: Get<Amount>;

        /// Maximum number of resources in one allocation.
        #[pallet::constant]
        type MaxResources: Get<u32>;

        /// Granularity of the expiration agenda. Must not be zero.
        #[pallet::constant]
        type ExpirationBucketSize: Get<MomentOf<Self>>;

        /// Maximum number of agenda entries in one bucket. Must not be zero.
        #[pallet::constant]
        type MaxExpirationsPerBucket: Get<u32>;

        /// Maximum amount of agenda work (scanned buckets plus processed
        /// entries) done in one block. Must not be zero.
        #[pallet::constant]
        type MaxExpirationsPerBlock: Get<u32>;

        /// Weight information.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Current subscription of every owner.
    #[pallet::storage]
    pub type Subscriptions<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, Subscription<MomentOf<T>>>;

    /// Last generation issued to an owner. Never removed, so generations
    /// stay unique across revocation and expiry.
    #[pallet::storage]
    pub type LastGeneration<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, ResourceGeneration, ValueQuery>;

    /// Resource entitlements of the current subscription.
    #[pallet::storage]
    pub type ResourceAllocations<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, Resource, Limit>;

    /// Global (owner-level) accounting state of the current subscription.
    #[pallet::storage]
    pub type GlobalResourceState<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        Resource,
        GenerationState<MomentOf<T>>,
    >;

    /// Local accounting state of `(owner, local context, resource)`.
    #[pallet::storage]
    pub type LocalResourceState<T: Config> = StorageNMap<
        _,
        (
            NMapKey<Blake2_128Concat, T::AccountId>,
            NMapKey<Blake2_128Concat, T::LocalId>,
            NMapKey<Blake2_128Concat, Resource>,
        ),
        GenerationState<MomentOf<T>>,
    >;

    /// Sum of the Transaction rates of all current subscriptions.
    ///
    /// Invariant: never exceeds `MaxSubscriptionTransactionRate`.
    #[pallet::storage]
    pub type TotalAllocatedTransactionRate<T: Config> = StorageValue<_, Amount, ValueQuery>;

    /// Subscriptions scheduled to expire, by time bucket.
    #[pallet::storage]
    pub type ExpirationAgenda<T: Config> = StorageMap<
        _,
        Twox64Concat,
        MomentOf<T>,
        BoundedVec<(T::AccountId, ResourceGeneration), T::MaxExpirationsPerBucket>,
        ValueQuery,
    >;

    /// First agenda bucket that may still hold unprocessed entries.
    #[pallet::storage]
    pub type NextExpirationBucket<T: Config> = StorageValue<_, MomentOf<T>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new subscription generation was allocated.
        SubscriptionAllocated {
            owner: T::AccountId,
            generation: ResourceGeneration,
            started_at: MomentOf<T>,
            expires_at: MomentOf<T>,
        },
        /// An allocation replaced the previous subscription generation.
        SubscriptionReplaced {
            owner: T::AccountId,
            previous_generation: ResourceGeneration,
            new_generation: ResourceGeneration,
        },
        /// A subscription expired and its global capacity was released.
        SubscriptionExpired {
            owner: T::AccountId,
            generation: ResourceGeneration,
        },
        /// A subscription was revoked and its global capacity was released.
        SubscriptionRevoked {
            owner: T::AccountId,
            generation: ResourceGeneration,
        },
        /// A resource was allocated as part of a subscription generation.
        ResourceAllocated {
            owner: T::AccountId,
            generation: ResourceGeneration,
            resource: Resource,
            allocation: Limit,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// The owner has no subscription.
        NoSubscription,
        /// The subscription is expired.
        SubscriptionExpired,
        /// The generation is not the current generation of the owner.
        WrongGeneration,
        /// The requested validity duration is zero.
        ZeroDuration,
        /// The expiration time does not fit the clock type.
        TimeOverflow,
        /// The resource is not supported by the pallet.
        UnknownResource,
        /// The resource allocation is malformed.
        InvalidResourceAllocation,
        /// A resource appears more than once in an allocation.
        DuplicateResource,
        /// The allocation has more resources than allowed.
        TooManyResources,
        /// The allocation would exceed the global Transaction rate capacity.
        GlobalRateCapacityExceeded,
        /// The resource is not part of the current subscription.
        ResourceNotAllocated,
        /// Accounting state of the resource is not available.
        ResourceUnavailable,
        /// No local limit is set for the local context and resource.
        MissingLocalLimit,
        /// The local bucket has not enough available amount.
        LocalLimitExceeded,
        /// The global bucket has not enough available amount.
        GlobalLimitExceeded,
        /// The charge receipt does not match the accounting state.
        InvalidCharge,
        /// The usage does not belong to the resource it is charged to.
        UsageMismatch,
        /// No expiration agenda bucket with free space is available.
        ExpirationAgendaFull,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            Self::process_expirations()
        }

        fn integrity_test() {
            assert!(
                !T::ExpirationBucketSize::get().is_zero(),
                "ExpirationBucketSize must not be zero"
            );
            assert!(
                T::MaxExpirationsPerBucket::get() > 0,
                "MaxExpirationsPerBucket must not be zero"
            );
            assert!(
                T::MaxExpirationsPerBlock::get() > 0,
                "MaxExpirationsPerBlock must not be zero"
            );
        }
    }

    impl<T: Config> Pallet<T> {
        /// Transaction rate allocated to `owner`, or zero.
        pub fn allocated_transaction_rate(owner: &T::AccountId) -> Amount {
            match ResourceAllocations::<T>::get(owner, Resource::Transaction) {
                Some(Limit::Transaction(rate)) => rate,
                None => 0,
            }
        }

        /// Maximum amount a bucket refilled at `rate` can accumulate.
        pub fn burst_capacity(rate: Amount) -> Amount {
            let period: u128 = T::TransactionBurstPeriod::get().unique_saturated_into();
            rate.saturating_mul(period)
        }

        /// Subscription of `owner` and its generation, if it is active.
        fn active_subscription(
            owner: &T::AccountId,
            generation: ResourceGeneration,
        ) -> Result<Subscription<MomentOf<T>>, DispatchError> {
            let subscription = Subscriptions::<T>::get(owner).ok_or(Error::<T>::NoSubscription)?;
            ensure!(
                subscription.generation == generation,
                Error::<T>::WrongGeneration
            );
            ensure!(
                T::Time::now() < subscription.expires_at,
                Error::<T>::SubscriptionExpired
            );
            Ok(subscription)
        }

        /// Bring `bucket` up to date with the elapsed time.
        fn refill(
            bucket: &RateState<MomentOf<T>>,
            rate: Amount,
            now: MomentOf<T>,
        ) -> RateState<MomentOf<T>> {
            let elapsed: u128 = now
                .saturating_sub(bucket.updated_at)
                .unique_saturated_into();
            let available = bucket
                .available
                .saturating_add(rate.saturating_mul(elapsed))
                .min(Self::burst_capacity(rate));
            RateState {
                available,
                updated_at: now,
            }
        }

        /// Global Transaction rate and bucket of `owner` refilled to `now`.
        fn refilled_global(
            owner: &T::AccountId,
            generation: ResourceGeneration,
            now: MomentOf<T>,
        ) -> Result<(Amount, RateState<MomentOf<T>>), DispatchError> {
            let Limit::Transaction(rate) =
                ResourceAllocations::<T>::get(owner, Resource::Transaction)
                    .ok_or(Error::<T>::ResourceNotAllocated)?;
            let state = GlobalResourceState::<T>::get(owner, Resource::Transaction)
                .filter(|state| state.generation == generation)
                .ok_or(Error::<T>::ResourceUnavailable)?;
            let ResourceState::Rate(bucket) = state.state;
            Ok((rate, Self::refill(&bucket, rate, now)))
        }

        /// Local Transaction rate of `local`.
        fn local_rate(local: &T::LocalId) -> Result<Amount, DispatchError> {
            match T::ResourceLimits::limit(local, Resource::Transaction)
                .ok_or(Error::<T>::MissingLocalLimit)?
            {
                Limit::Transaction(rate) => Ok(rate),
            }
        }

        /// Local bucket of the current generation refilled to `now`.
        ///
        /// A missing bucket or a bucket of an older generation is logically
        /// empty and starts accruing at `started_at` of the current
        /// generation, like the global bucket. Starting it at `now` would make
        /// the first operation fail, and since a failed operation stores
        /// nothing, the bucket could never be initialized. The global bucket
        /// stays authoritative, so this cannot exceed the owner rate.
        fn refilled_local(
            owner: &T::AccountId,
            local: &T::LocalId,
            generation: ResourceGeneration,
            started_at: MomentOf<T>,
            rate: Amount,
            now: MomentOf<T>,
        ) -> RateState<MomentOf<T>> {
            let current =
                LocalResourceState::<T>::get((owner.clone(), local.clone(), Resource::Transaction))
                    .filter(|state| state.generation == generation)
                    .map(|state| {
                        let ResourceState::Rate(bucket) = state.state;
                        bucket
                    });
            match current {
                Some(bucket) => Self::refill(&bucket, rate, now),
                None => Self::refill(
                    &RateState {
                        available: 0,
                        updated_at: started_at,
                    },
                    rate,
                    now,
                ),
            }
        }

        fn put_global(
            owner: &T::AccountId,
            generation: ResourceGeneration,
            bucket: RateState<MomentOf<T>>,
        ) {
            GlobalResourceState::<T>::insert(
                owner,
                Resource::Transaction,
                GenerationState {
                    generation,
                    state: ResourceState::Rate(bucket),
                },
            );
        }

        fn put_local(
            owner: &T::AccountId,
            local: &T::LocalId,
            generation: ResourceGeneration,
            bucket: RateState<MomentOf<T>>,
        ) {
            LocalResourceState::<T>::insert(
                (owner.clone(), local.clone(), Resource::Transaction),
                GenerationState {
                    generation,
                    state: ResourceState::Rate(bucket),
                },
            );
        }

        /// Amount of a Transaction usage charged to `resource`.
        fn transaction_amount(resource: Resource, usage: Usage) -> Result<Amount, DispatchError> {
            ensure!(
                resource == Resource::Transaction,
                Error::<T>::UnknownResource
            );
            match usage {
                Usage::Transaction(amount) => Ok(amount),
            }
        }

        /// Remove the allocations and global state of `owner` and return the
        /// Transaction rate they held. Does not touch the aggregate.
        fn clear_allocations(owner: &T::AccountId) -> Amount {
            let rate = Self::allocated_transaction_rate(owner);
            let resources: Vec<Resource> =
                ResourceAllocations::<T>::iter_key_prefix(owner).collect();
            for resource in resources {
                ResourceAllocations::<T>::remove(owner, resource);
                GlobalResourceState::<T>::remove(owner, resource);
            }
            rate
        }

        /// Remove the subscription of `owner` and release its global
        /// capacity. Returns the removed subscription.
        fn release_subscription(owner: &T::AccountId) -> Option<Subscription<MomentOf<T>>> {
            let subscription = Subscriptions::<T>::take(owner)?;
            let rate = Self::clear_allocations(owner);
            TotalAllocatedTransactionRate::<T>::mutate(|total| *total = total.saturating_sub(rate));
            Some(subscription)
        }

        /// Add `(owner, generation)` to the agenda bucket at or after
        /// `expires_at`.
        fn schedule_expiration(
            owner: &T::AccountId,
            generation: ResourceGeneration,
            expires_at: MomentOf<T>,
        ) -> DispatchResult {
            let size = T::ExpirationBucketSize::get();
            let mut bucket = expires_at / size;
            if !(expires_at % size).is_zero() {
                bucket = bucket
                    .checked_add(&One::one())
                    .ok_or(Error::<T>::TimeOverflow)?;
            }
            for _ in 0..MAX_AGENDA_PROBES {
                let mut entries = ExpirationAgenda::<T>::get(bucket);
                if entries.try_push((owner.clone(), generation)).is_ok() {
                    ExpirationAgenda::<T>::insert(bucket, entries);
                    NextExpirationBucket::<T>::mutate(|next| {
                        if next.is_none_or(|next| bucket < next) {
                            *next = Some(bucket);
                        }
                    });
                    return Ok(());
                }
                bucket = bucket
                    .checked_add(&One::one())
                    .ok_or(Error::<T>::TimeOverflow)?;
            }
            Err(Error::<T>::ExpirationAgendaFull.into())
        }

        /// Release the subscription of `owner` if `generation` is still
        /// current and expired at `now`.
        fn expire(owner: &T::AccountId, generation: ResourceGeneration, now: MomentOf<T>) {
            let due = Subscriptions::<T>::get(owner)
                .is_some_and(|sub| sub.generation == generation && now >= sub.expires_at);
            if due {
                Self::release_subscription(owner);
                Self::deposit_event(Event::SubscriptionExpired {
                    owner: owner.clone(),
                    generation,
                });
            }
        }

        /// Process due agenda buckets within `MaxExpirationsPerBlock` work.
        fn process_expirations() -> Weight {
            let db = T::DbWeight::get();
            let Some(mut cursor) = NextExpirationBucket::<T>::get() else {
                return db.reads(1);
            };
            let now = T::Time::now();
            let last_due = now / T::ExpirationBucketSize::get();
            let mut budget = T::MaxExpirationsPerBlock::get();
            let mut weight = db.reads(1);
            let start = cursor;

            while cursor <= last_due && budget > 0 {
                budget -= 1;
                weight = weight.saturating_add(T::WeightInfo::scan_bucket());
                let mut entries = ExpirationAgenda::<T>::take(cursor).into_inner();
                let take = entries.len().min(budget as usize);
                let rest = entries.split_off(take);
                budget -= take as u32;
                for (owner, generation) in entries {
                    weight = weight.saturating_add(T::WeightInfo::expire_entry());
                    Self::expire(&owner, generation, now);
                }
                if !rest.is_empty() {
                    ExpirationAgenda::<T>::insert(cursor, BoundedVec::truncate_from(rest));
                    break;
                }
                cursor = cursor.saturating_add(One::one());
            }

            if cursor != start {
                NextExpirationBucket::<T>::put(cursor);
                weight = weight.saturating_add(db.writes(1));
            }
            weight
        }

        fn do_allocate(owner: &T::AccountId, allocation: AllocationOf<T>) -> DispatchResult {
            let started_at = T::Time::now();
            ensure!(!allocation.valid_for.is_zero(), Error::<T>::ZeroDuration);
            let expires_at = started_at
                .checked_add(&allocation.valid_for)
                .ok_or(Error::<T>::TimeOverflow)?;

            let mut new_rate: Amount = 0;
            for (index, item) in allocation.resources.iter().enumerate() {
                ensure!(
                    !allocation.resources[..index]
                        .iter()
                        .any(|previous| previous.resource == item.resource),
                    Error::<T>::DuplicateResource
                );
                ensure!(
                    item.resource == Resource::Transaction,
                    Error::<T>::UnknownResource
                );
                let Limit::Transaction(rate) = item.limit;
                ensure!(
                    item.limit.resource() == item.resource && rate > 0,
                    Error::<T>::InvalidResourceAllocation
                );
                new_rate = rate;
            }

            let previous = Subscriptions::<T>::get(owner);
            let next_total = TotalAllocatedTransactionRate::<T>::get()
                .saturating_sub(Self::allocated_transaction_rate(owner))
                .checked_add(new_rate)
                .ok_or(Error::<T>::GlobalRateCapacityExceeded)?;
            ensure!(
                next_total <= T::MaxSubscriptionTransactionRate::get(),
                Error::<T>::GlobalRateCapacityExceeded
            );
            let generation = LastGeneration::<T>::get(owner)
                .checked_add(1)
                .ok_or(ArithmeticError::Overflow)?;

            Self::schedule_expiration(owner, generation, expires_at)?;
            Self::clear_allocations(owner);
            TotalAllocatedTransactionRate::<T>::put(next_total);
            LastGeneration::<T>::insert(owner, generation);
            Subscriptions::<T>::insert(
                owner,
                Subscription {
                    generation,
                    started_at,
                    expires_at,
                },
            );
            for item in allocation.resources.iter() {
                ResourceAllocations::<T>::insert(owner, item.resource, item.limit);
                Self::put_global(
                    owner,
                    generation,
                    RateState {
                        available: 0,
                        updated_at: started_at,
                    },
                );
            }

            Self::deposit_event(Event::SubscriptionAllocated {
                owner: owner.clone(),
                generation,
                started_at,
                expires_at,
            });
            if let Some(previous) = previous {
                Self::deposit_event(Event::SubscriptionReplaced {
                    owner: owner.clone(),
                    previous_generation: previous.generation,
                    new_generation: generation,
                });
            }
            for item in allocation.resources.iter() {
                Self::deposit_event(Event::ResourceAllocated {
                    owner: owner.clone(),
                    generation,
                    resource: item.resource,
                    allocation: item.limit,
                });
            }
            Ok(())
        }

        /// Return `charged - actual` to `bucket`, or take the extra
        /// `actual - charged`, within the burst capacity of `rate`.
        fn apply_correction(
            bucket: &mut RateState<MomentOf<T>>,
            rate: Amount,
            charged: Amount,
            actual: Amount,
        ) {
            bucket.available = if actual <= charged {
                bucket
                    .available
                    .saturating_add(charged - actual)
                    .min(Self::burst_capacity(rate))
            } else {
                bucket.available.saturating_sub(actual - charged)
            };
        }

        fn do_correct(
            charge: Charge<T::AccountId, T::LocalId>,
            actual_usage: Usage,
        ) -> DispatchResult {
            let ResourceCharge {
                owner,
                generation,
                resource,
                source,
                charged,
            } = charge.0;
            let charged = Self::transaction_amount(resource, charged)?;
            ensure!(
                actual_usage.resource() == resource,
                Error::<T>::UsageMismatch
            );
            let Usage::Transaction(actual) = actual_usage;

            let subscription = Subscriptions::<T>::get(&owner).ok_or(Error::<T>::NoSubscription)?;
            ensure!(
                subscription.generation == generation,
                Error::<T>::WrongGeneration
            );

            let now = T::Time::now();
            let (rate, mut global) = Self::refilled_global(&owner, generation, now)?;
            Self::apply_correction(&mut global, rate, charged, actual);

            if let ChargeSource::Local(local) = &source {
                let state = LocalResourceState::<T>::get((
                    owner.clone(),
                    local.clone(),
                    Resource::Transaction,
                ))
                .filter(|state| state.generation == generation)
                .ok_or(Error::<T>::InvalidCharge)?;
                let ResourceState::Rate(bucket) = state.state;
                // A removed local limit leaves the local bucket untouched.
                if let Ok(local_rate) = Self::local_rate(local) {
                    let mut local_bucket = Self::refill(&bucket, local_rate, now);
                    Self::apply_correction(&mut local_bucket, local_rate, charged, actual);
                    Self::put_local(&owner, local, generation, local_bucket);
                }
            }
            Self::put_global(&owner, generation, global);
            Ok(())
        }
    }

    impl<T: Config> SubscriptionAllocator<T::AccountId> for Pallet<T> {
        type ResourceId = Resource;
        type Limit = Limit;
        type Moment = MomentOf<T>;
        type MaxResources = T::MaxResources;

        /// Replace the subscription of `owner` with a new generation.
        ///
        /// Atomic: on error neither the subscription, the allocations nor
        /// the aggregate rate change.
        fn allocate(owner: &T::AccountId, allocation: AllocationOf<T>) -> DispatchResult {
            with_storage_layer(|| Self::do_allocate(owner, allocation))
        }

        fn revoke(owner: &T::AccountId) -> DispatchResult {
            let subscription =
                Self::release_subscription(owner).ok_or(Error::<T>::NoSubscription)?;
            Self::deposit_event(Event::SubscriptionRevoked {
                owner: owner.clone(),
                generation: subscription.generation,
            });
            Ok(())
        }
    }

    impl<T: Config> ResourceAccounting<T::AccountId> for Pallet<T> {
        type LocalId = T::LocalId;
        type ResourceId = Resource;
        type Usage = Usage;
        type Generation = ResourceGeneration;
        type Charge = Charge<T::AccountId, T::LocalId>;

        /// Generation of the subscription of `owner` if it is active.
        fn generation(owner: &T::AccountId) -> Option<ResourceGeneration> {
            let now = T::Time::now();
            Subscriptions::<T>::get(owner)
                .filter(|subscription| now < subscription.expires_at)
                .map(|subscription| subscription.generation)
        }

        fn consume_global(
            owner: &T::AccountId,
            generation: ResourceGeneration,
            resource: Resource,
            usage: Usage,
        ) -> Result<Self::Charge, DispatchError> {
            let amount = Self::transaction_amount(resource, usage)?;
            ensure!(usage.resource() == resource, Error::<T>::UsageMismatch);
            Self::active_subscription(owner, generation)?;

            let now = T::Time::now();
            let (_, mut global) = Self::refilled_global(owner, generation, now)?;
            ensure!(global.available >= amount, Error::<T>::GlobalLimitExceeded);
            global.available -= amount;
            Self::put_global(owner, generation, global);

            Ok(Charge(ResourceCharge {
                owner: owner.clone(),
                generation,
                resource,
                source: ChargeSource::Global,
                charged: usage,
            }))
        }

        fn consume_local(
            owner: &T::AccountId,
            local: &T::LocalId,
            generation: ResourceGeneration,
            resource: Resource,
            usage: Usage,
        ) -> Result<Self::Charge, DispatchError> {
            let amount = Self::transaction_amount(resource, usage)?;
            let subscription = Self::active_subscription(owner, generation)?;

            let now = T::Time::now();
            let (_, mut global) = Self::refilled_global(owner, generation, now)?;
            let local_rate = Self::local_rate(local)?;
            let mut local_bucket = Self::refilled_local(
                owner,
                local,
                generation,
                subscription.started_at,
                local_rate,
                now,
            );
            ensure!(
                local_bucket.available >= amount,
                Error::<T>::LocalLimitExceeded
            );
            ensure!(global.available >= amount, Error::<T>::GlobalLimitExceeded);
            local_bucket.available -= amount;
            global.available -= amount;
            Self::put_local(owner, local, generation, local_bucket);
            Self::put_global(owner, generation, global);

            Ok(Charge(ResourceCharge {
                owner: owner.clone(),
                generation,
                resource,
                source: ChargeSource::Local(local.clone()),
                charged: usage,
            }))
        }

        /// Replace the charged usage with the actual one in the generation
        /// recorded in the charge. Global and local state are corrected
        /// atomically; a newer generation is never affected.
        fn correct(charge: Self::Charge, actual_usage: Usage) -> DispatchResult {
            with_storage_layer(|| Self::do_correct(charge, actual_usage))
        }
    }
}
