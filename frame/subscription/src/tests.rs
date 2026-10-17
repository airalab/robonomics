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
//! Tests for pallet-robonomics-subscription

use crate::{self as pallet_subscription, *};
use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types, traits::Hooks, BoundedVec,
};
use robonomics_subscription_primitives::{
    ChargeSource, Resource, ResourceAccounting, ResourceAllocation, ResourceGeneration,
    ResourceLimit, SubscriptionAllocation, SubscriptionAllocator,
};
use sp_runtime::BuildStorage;
use std::{cell::RefCell, collections::BTreeMap};

type Block = frame_system::mocking::MockBlock<Runtime>;

frame_support::construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Subscription: pallet_subscription,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type Block = Block;
    type AccountData = ();
}

thread_local! {
    static NOW: RefCell<u64> = const { RefCell::new(0) };
    static LIMITS: RefCell<BTreeMap<u32, Amount>> = const { RefCell::new(BTreeMap::new()) };
}

/// Runtime clock controlled by the tests, independent from block numbers.
pub struct MockTime;

impl frame_support::traits::Time for MockTime {
    type Moment = u64;

    fn now() -> u64 {
        NOW.with(|now| *now.borrow())
    }
}

/// Local limits controlled by the tests.
pub struct MockLimits;

impl ResourceLimit for MockLimits {
    type LocalId = u32;
    type ResourceId = Resource;
    type Limit = Limit;

    fn limit(local: &u32, resource: Resource) -> Option<Limit> {
        match resource {
            Resource::Transaction => LIMITS
                .with(|limits| limits.borrow().get(local).copied())
                .map(Limit::Transaction),
            _ => None,
        }
    }
}

parameter_types! {
    pub const BurstPeriod: u64 = 2_000;
    pub const MaxRate: Amount = 100;
    pub const BucketSize: u64 = 1_000;
    pub const MaxPerBucket: u32 = 3;
    pub const MaxPerBlock: u32 = 8;
}

impl pallet_subscription::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Time = MockTime;
    type LocalId = u32;
    type ResourceLimits = MockLimits;
    type TransactionBurstPeriod = BurstPeriod;
    type MaxSubscriptionTransactionRate = MaxRate;
    type MaxResources = frame_support::traits::ConstU32<3>;
    type ExpirationBucketSize = BucketSize;
    type MaxExpirationsPerBucket = MaxPerBucket;
    type MaxExpirationsPerBlock = MaxPerBlock;
    type WeightInfo = ();
}

const ALICE: u64 = 1;
const BOB: u64 = 2;
const CAROL: u64 = 3;

fn new_test_ext() -> sp_io::TestExternalities {
    NOW.with(|now| *now.borrow_mut() = 10_000);
    LIMITS.with(|limits| limits.borrow_mut().clear());
    let storage = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(storage);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn set_time(now: u64) {
    NOW.with(|value| *value.borrow_mut() = now);
}

fn advance_time(by: u64) {
    NOW.with(|value| *value.borrow_mut() += by);
}

fn set_local_limit(local: u32, rate: Amount) {
    LIMITS.with(|limits| limits.borrow_mut().insert(local, rate));
}

fn allocation_of(
    valid_for: u64,
    resources: Vec<(Resource, Limit)>,
) -> SubscriptionAllocation<Resource, Limit, u64, frame_support::traits::ConstU32<3>> {
    SubscriptionAllocation {
        valid_for,
        resources: BoundedVec::truncate_from(
            resources
                .into_iter()
                .map(|(resource, limit)| ResourceAllocation { resource, limit })
                .collect(),
        ),
    }
}

fn allocate(owner: u64, valid_for: u64, rate: Amount) -> frame_support::dispatch::DispatchResult {
    Subscription::allocate(
        &owner,
        allocation_of(
            valid_for,
            vec![(Resource::Transaction, Limit::Transaction(rate))],
        ),
    )
}

fn generation(owner: u64) -> ResourceGeneration {
    Subscriptions::<Runtime>::get(owner).unwrap().generation
}

fn global_bucket(owner: u64) -> RateState<u64> {
    let ResourceState::Rate(bucket) =
        GlobalResourceState::<Runtime>::get(owner, Resource::Transaction)
            .unwrap()
            .state;
    bucket
}

fn local_bucket(owner: u64, local: u32) -> GenerationState<u64> {
    LocalResourceState::<Runtime>::get((owner, local, Resource::Transaction)).unwrap()
}

fn consume_global(
    owner: u64,
    amount: Amount,
) -> Result<Charge<u64, u32>, sp_runtime::DispatchError> {
    Subscription::consume_global(
        &owner,
        generation(owner),
        Resource::Transaction,
        Usage::Transaction(amount),
    )
}

fn consume_local(
    owner: u64,
    local: u32,
    amount: Amount,
) -> Result<Charge<u64, u32>, sp_runtime::DispatchError> {
    Subscription::consume_local(
        &owner,
        &local,
        generation(owner),
        Resource::Transaction,
        Usage::Transaction(amount),
    )
}

fn events() -> Vec<pallet_subscription::Event<Runtime>> {
    System::events()
        .into_iter()
        .filter_map(|record| match record.event {
            RuntimeEvent::Subscription(event) => Some(event),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Time
// ---------------------------------------------------------------------------

#[test]
fn allocation_uses_runtime_time() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 5_000, 5));
        let subscription = Subscriptions::<Runtime>::get(ALICE).unwrap();
        assert_eq!(subscription.started_at, 10_000);
        assert_eq!(subscription.expires_at, 15_000);
    });
}

#[test]
fn zero_duration_is_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(allocate(ALICE, 0, 5), Error::<Runtime>::ZeroDuration);
    });
}

#[test]
fn time_overflow_is_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(allocate(ALICE, u64::MAX, 5), Error::<Runtime>::TimeOverflow);
    });
}

#[test]
fn subscription_expires_exactly_at_expires_at() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 5_000, 5));
        set_time(14_999);
        assert_eq!(Subscription::generation(&ALICE), Some(1));
        set_time(15_000);
        assert_eq!(Subscription::generation(&ALICE), None);
        assert_noop!(
            Subscription::consume_global(&ALICE, 1, Resource::Transaction, Usage::Transaction(0)),
            Error::<Runtime>::SubscriptionExpired
        );
    });
}

#[test]
fn block_number_does_not_expire_subscription() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 5_000, 5));
        System::set_block_number(1_000_000);
        assert_eq!(Subscription::generation(&ALICE), Some(1));
        Subscription::on_initialize(1_000_000);
        assert!(Subscriptions::<Runtime>::contains_key(ALICE));
    });
}

#[test]
fn time_advance_expires_subscription_without_new_blocks() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 5_000, 5));
        set_time(20_000);
        assert_eq!(Subscription::generation(&ALICE), None);
    });
}

// ---------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------

#[test]
fn first_allocation_creates_generation_and_replacement_increments_it() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 5_000, 5));
        assert_eq!(generation(ALICE), 1);
        assert_ok!(allocate(ALICE, 5_000, 5));
        assert_eq!(generation(ALICE), 2);
        assert_eq!(
            events()
                .into_iter()
                .filter(|event| matches!(
                    event,
                    Event::SubscriptionReplaced {
                        previous_generation: 1,
                        new_generation: 2,
                        ..
                    }
                ))
                .count(),
            1
        );
    });
}

#[test]
fn generation_survives_revocation() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 5_000, 5));
        assert_ok!(Subscription::revoke(&ALICE));
        assert_ok!(allocate(ALICE, 5_000, 5));
        assert_eq!(generation(ALICE), 2);
    });
}

#[test]
fn old_generation_cannot_consume() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 5_000, 5));
        assert_ok!(allocate(ALICE, 5_000, 5));
        assert_noop!(
            Subscription::consume_global(&ALICE, 1, Resource::Transaction, Usage::Transaction(0)),
            Error::<Runtime>::WrongGeneration
        );
    });
}

#[test]
fn old_generation_cannot_receive_correction() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(1_000);
        let charge = consume_global(ALICE, 1_000).unwrap();
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(100);
        let before = global_bucket(ALICE);
        assert_noop!(
            Subscription::correct(charge, Usage::Transaction(0)),
            Error::<Runtime>::WrongGeneration
        );
        assert_eq!(global_bucket(ALICE), before);
    });
}

#[test]
fn old_generation_local_state_is_lazily_invalidated() {
    new_test_ext().execute_with(|| {
        set_local_limit(7, 5);
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(1_000);
        assert_ok!(consume_local(ALICE, 7, 1_000));
        assert_eq!(local_bucket(ALICE, 7).generation, 1);
        assert_ok!(allocate(ALICE, 60_000, 5));
        // The old state is untouched until the first use in the new
        // generation, which starts empty.
        assert_eq!(local_bucket(ALICE, 7).generation, 1);
        assert_noop!(
            consume_local(ALICE, 7, 1),
            Error::<Runtime>::LocalLimitExceeded
        );
        advance_time(100);
        assert_ok!(consume_local(ALICE, 7, 500));
        let state = local_bucket(ALICE, 7);
        assert_eq!(state.generation, 2);
        assert_eq!(
            state.state,
            ResourceState::Rate(RateState {
                available: 0,
                updated_at: 11_100
            })
        );
    });
}

// ---------------------------------------------------------------------------
// Global Transaction rate
// ---------------------------------------------------------------------------

#[test]
fn allocation_updates_total_and_respects_capacity() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 5_000, 60));
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 60);
        assert_ok!(allocate(BOB, 5_000, 40));
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 100);
        assert_noop!(
            allocate(CAROL, 5_000, 1),
            Error::<Runtime>::GlobalRateCapacityExceeded
        );
        assert!(!Subscriptions::<Runtime>::contains_key(CAROL));
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 100);
    });
}

#[test]
fn replacement_accounts_only_rate_delta() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 5_000, 60));
        assert_ok!(allocate(BOB, 5_000, 30));
        // 90 - 60 + 70 = 100 fits, although 90 + 70 would not.
        assert_ok!(allocate(ALICE, 5_000, 70));
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 100);
        assert_ok!(allocate(ALICE, 5_000, 10));
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 40);
    });
}

#[test]
fn failed_replacement_is_atomic() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 5_000, 60));
        assert_ok!(allocate(BOB, 5_000, 30));
        let subscription = Subscriptions::<Runtime>::get(ALICE);
        assert_noop!(
            allocate(ALICE, 9_000, 80),
            Error::<Runtime>::GlobalRateCapacityExceeded
        );
        assert_eq!(Subscriptions::<Runtime>::get(ALICE), subscription);
        assert_eq!(Subscription::allocated_transaction_rate(&ALICE), 60);
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 90);
    });
}

#[test]
fn full_agenda_fails_allocation_atomically() {
    new_test_ext().execute_with(|| {
        // All buckets MAX_AGENDA_PROBES deep from 15_000 are filled.
        for bucket in 15..15 + crate::MAX_AGENDA_PROBES as u64 {
            let entries = (0..MaxPerBucket::get() as u64)
                .map(|i| (100 + i, 1))
                .collect();
            ExpirationAgenda::<Runtime>::insert(bucket, BoundedVec::truncate_from(entries));
        }
        assert_noop!(
            allocate(ALICE, 5_000, 5),
            Error::<Runtime>::ExpirationAgendaFull
        );
        assert!(!Subscriptions::<Runtime>::contains_key(ALICE));
        assert_eq!(LastGeneration::<Runtime>::get(ALICE), 0);
    });
}

#[test]
fn expiration_releases_global_capacity() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 5_000, 60));
        set_time(15_000);
        Subscription::on_initialize(2);
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 0);
        assert!(!Subscriptions::<Runtime>::contains_key(ALICE));
        assert!(ResourceAllocations::<Runtime>::get(ALICE, Resource::Transaction).is_none());
        assert!(events().contains(&Event::SubscriptionExpired {
            owner: ALICE,
            generation: 1
        }));
        assert_ok!(allocate(BOB, 5_000, 100));
    });
}

#[test]
fn expiration_is_not_processed_before_expiry() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 5_500, 60));
        // Scheduled in bucket 16 (ceil of 15_500 / 1_000).
        set_time(15_999);
        Subscription::on_initialize(2);
        assert!(Subscriptions::<Runtime>::contains_key(ALICE));
        set_time(16_000);
        Subscription::on_initialize(3);
        assert!(!Subscriptions::<Runtime>::contains_key(ALICE));
    });
}

#[test]
fn stale_expiration_entry_cannot_release_replacement_capacity() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 5_000, 60));
        assert_ok!(allocate(ALICE, 50_000, 60));
        set_time(16_000);
        Subscription::on_initialize(2);
        assert_eq!(generation(ALICE), 2);
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 60);
        assert_eq!(Subscription::allocated_transaction_rate(&ALICE), 60);
    });
}

#[test]
fn revocation_releases_global_capacity() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 5_000, 60));
        assert_ok!(Subscription::revoke(&ALICE));
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 0);
        assert_noop!(
            Subscription::revoke(&ALICE),
            Error::<Runtime>::NoSubscription
        );
        assert!(events().contains(&Event::SubscriptionRevoked {
            owner: ALICE,
            generation: 1
        }));
        // The agenda entry of the revoked generation does nothing.
        set_time(20_000);
        Subscription::on_initialize(2);
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 0);
    });
}

#[test]
fn expiration_processing_is_bounded_and_catches_up() {
    new_test_ext().execute_with(|| {
        // MaxPerBlock = 8: one block can do at most 8 units of agenda work.
        for owner in 10..16u64 {
            assert_ok!(allocate(owner, 5_000, 1));
        }
        set_time(100_000);
        Subscription::on_initialize(2);
        let remaining = (10..16u64)
            .filter(|owner| Subscriptions::<Runtime>::contains_key(owner))
            .count();
        assert!(remaining <= 6);
        for block in 3..10 {
            Subscription::on_initialize(block);
        }
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 0);
        assert!((10..16u64).all(|owner| !Subscriptions::<Runtime>::contains_key(owner)));
    });
}

#[test]
fn full_bucket_spills_into_next_bucket() {
    new_test_ext().execute_with(|| {
        for owner in 10..14u64 {
            assert_ok!(allocate(owner, 5_000, 1));
        }
        assert_eq!(ExpirationAgenda::<Runtime>::get(15).len(), 3);
        assert_eq!(ExpirationAgenda::<Runtime>::get(16).len(), 1);
    });
}

// ---------------------------------------------------------------------------
// Resource validation
// ---------------------------------------------------------------------------

#[test]
fn invalid_allocations_are_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Subscription::allocate(
                &ALICE,
                allocation_of(
                    5_000,
                    vec![
                        (Resource::Transaction, Limit::Transaction(1)),
                        (Resource::Transaction, Limit::Transaction(2)),
                    ]
                )
            ),
            Error::<Runtime>::DuplicateResource
        );
        assert_noop!(
            allocate(ALICE, 5_000, 0),
            Error::<Runtime>::InvalidResourceAllocation
        );
        assert_noop!(
            Subscription::allocate(
                &ALICE,
                allocation_of(5_000, vec![(Resource::Storage, Limit::Transaction(1))])
            ),
            Error::<Runtime>::UnknownResource
        );
    });
}

#[test]
fn allocation_without_transaction_resource_is_valid() {
    new_test_ext().execute_with(|| {
        assert_ok!(Subscription::allocate(&ALICE, allocation_of(5_000, vec![])));
        assert_eq!(Subscription::allocated_transaction_rate(&ALICE), 0);
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 0);
        assert_noop!(
            consume_global(ALICE, 0),
            Error::<Runtime>::ResourceNotAllocated
        );
    });
}

#[test]
fn allocation_emits_events() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 5_000, 5));
        let events = events();
        assert!(events.contains(&Event::SubscriptionAllocated {
            owner: ALICE,
            generation: 1,
            started_at: 10_000,
            expires_at: 15_000
        }));
        assert!(events.contains(&Event::ResourceAllocated {
            owner: ALICE,
            generation: 1,
            resource: Resource::Transaction,
            allocation: Limit::Transaction(5)
        }));
    });
}

// ---------------------------------------------------------------------------
// Transaction token bucket
// ---------------------------------------------------------------------------

#[test]
fn new_generation_starts_with_zero_tokens() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(1_000);
        assert_ok!(consume_global(ALICE, 4_000));
        assert_ok!(allocate(ALICE, 60_000, 5));
        let bucket = global_bucket(ALICE);
        assert_eq!(bucket.available, 0);
        assert_eq!(bucket.updated_at, 10_000 + 1_000);
        assert_noop!(
            consume_global(ALICE, 1),
            Error::<Runtime>::GlobalLimitExceeded
        );
    });
}

#[test]
fn tokens_refill_by_elapsed_time_not_block_number() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 60_000, 5));
        System::set_block_number(500);
        assert_noop!(
            consume_global(ALICE, 1),
            Error::<Runtime>::GlobalLimitExceeded
        );
        advance_time(100);
        // 5 Wn/ms * 100 ms.
        assert_ok!(consume_global(ALICE, 500));
        assert_noop!(
            consume_global(ALICE, 1),
            Error::<Runtime>::GlobalLimitExceeded
        );
        System::set_block_number(501);
        assert_noop!(
            consume_global(ALICE, 1),
            Error::<Runtime>::GlobalLimitExceeded
        );
    });
}

#[test]
fn refill_is_capped_by_burst_period() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 600_000, 5));
        // Cap is 5 * 2_000 = 10_000 after a long idle period.
        advance_time(500_000);
        assert_noop!(
            consume_global(ALICE, 10_001),
            Error::<Runtime>::GlobalLimitExceeded
        );
        assert_ok!(consume_global(ALICE, 10_000));
        assert_eq!(global_bucket(ALICE).available, 0);
    });
}

#[test]
fn consumption_reduces_available_tokens() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(1_000);
        assert_ok!(consume_global(ALICE, 1_500));
        assert_eq!(global_bucket(ALICE).available, 3_500);
        assert_ok!(consume_global(ALICE, 3_500));
        assert_noop!(
            consume_global(ALICE, 1),
            Error::<Runtime>::GlobalLimitExceeded
        );
    });
}

#[test]
fn failed_consumption_does_not_change_state() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(100);
        let before = global_bucket(ALICE);
        assert_noop!(
            consume_global(ALICE, 501),
            Error::<Runtime>::GlobalLimitExceeded
        );
        assert_eq!(global_bucket(ALICE), before);
    });
}

#[test]
fn usage_must_match_resource() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 60_000, 5));
        assert_noop!(
            Subscription::consume_global(&ALICE, 1, Resource::Storage, Usage::Transaction(0)),
            Error::<Runtime>::UnknownResource
        );
        assert_noop!(
            Subscription::consume_global(&BOB, 1, Resource::Transaction, Usage::Transaction(0)),
            Error::<Runtime>::NoSubscription
        );
    });
}

// ---------------------------------------------------------------------------
// Local Transaction accounting
// ---------------------------------------------------------------------------

#[test]
fn missing_local_limit_rejects_local_usage() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(1_000);
        assert_noop!(
            consume_local(ALICE, 7, 0),
            Error::<Runtime>::MissingLocalLimit
        );
        assert_eq!(global_bucket(ALICE).available, 0);
    });
}

#[test]
fn local_bucket_refills_by_local_rate_and_is_burst_capped() {
    new_test_ext().execute_with(|| {
        set_local_limit(7, 3);
        assert_ok!(allocate(ALICE, 600_000, 10));
        advance_time(100);
        // Local: 3 * 100 = 300, global: 10 * 100 = 1_000.
        assert_noop!(
            consume_local(ALICE, 7, 301),
            Error::<Runtime>::LocalLimitExceeded
        );
        assert_ok!(consume_local(ALICE, 7, 300));
        // Idle for long: local cap 3 * 2_000 = 6_000, global cap 20_000.
        advance_time(500_000);
        assert_noop!(
            consume_local(ALICE, 7, 6_001),
            Error::<Runtime>::LocalLimitExceeded
        );
        assert_ok!(consume_local(ALICE, 7, 6_000));
    });
}

#[test]
fn local_and_global_buckets_are_charged_atomically() {
    new_test_ext().execute_with(|| {
        set_local_limit(7, 4);
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(100);
        assert_ok!(consume_local(ALICE, 7, 100));
        assert_eq!(global_bucket(ALICE).available, 400);
        let ResourceState::Rate(local) = local_bucket(ALICE, 7).state;
        assert_eq!(local.available, 300);
    });
}

#[test]
fn failing_local_check_leaves_both_buckets_unchanged() {
    new_test_ext().execute_with(|| {
        set_local_limit(7, 3);
        assert_ok!(allocate(ALICE, 60_000, 10));
        advance_time(100);
        assert_ok!(consume_local(ALICE, 7, 10));
        let global = global_bucket(ALICE);
        let local = local_bucket(ALICE, 7);
        advance_time(0);
        assert_noop!(
            consume_local(ALICE, 7, 291),
            Error::<Runtime>::LocalLimitExceeded
        );
        assert_eq!(global_bucket(ALICE), global);
        assert_eq!(local_bucket(ALICE, 7), local);
    });
}

#[test]
fn failing_global_check_leaves_local_bucket_unchanged() {
    new_test_ext().execute_with(|| {
        // Local rate overcommits the global one.
        set_local_limit(7, 50);
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(100);
        assert_noop!(
            consume_local(ALICE, 7, 501),
            Error::<Runtime>::GlobalLimitExceeded
        );
        assert!(LocalResourceState::<Runtime>::get((ALICE, 7u32, Resource::Transaction)).is_none());
    });
}

#[test]
fn local_overcommit_is_allowed_and_global_rate_is_authoritative() {
    new_test_ext().execute_with(|| {
        set_local_limit(1, 4);
        set_local_limit(2, 4);
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(100);
        // Local rates sum to 8 > global 5; each local bucket has 400.
        assert_ok!(consume_local(ALICE, 1, 300));
        assert_noop!(
            consume_local(ALICE, 2, 300),
            Error::<Runtime>::GlobalLimitExceeded
        );
        assert_ok!(consume_local(ALICE, 2, 200));
    });
}

#[test]
fn local_state_is_per_owner_and_local_context() {
    new_test_ext().execute_with(|| {
        set_local_limit(1, 4);
        set_local_limit(2, 4);
        assert_ok!(allocate(ALICE, 60_000, 10));
        assert_ok!(allocate(BOB, 60_000, 10));
        advance_time(100);
        assert_ok!(consume_local(ALICE, 1, 400));
        assert_ok!(consume_local(ALICE, 2, 400));
        assert_ok!(consume_local(BOB, 1, 400));
    });
}

// ---------------------------------------------------------------------------
// Correction
// ---------------------------------------------------------------------------

#[test]
fn global_charge_is_corrected_to_actual_usage() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(1_000);
        let charge = consume_global(ALICE, 3_000).unwrap();
        assert_eq!(global_bucket(ALICE).available, 2_000);
        assert_ok!(Subscription::correct(
            charge.clone(),
            Usage::Transaction(1_000)
        ));
        assert_eq!(global_bucket(ALICE).available, 4_000);

        let charge = consume_global(ALICE, 1_000).unwrap();
        assert_ok!(Subscription::correct(charge, Usage::Transaction(1_500)));
        assert_eq!(global_bucket(ALICE).available, 2_500);
    });
}

#[test]
fn refund_is_capped_by_burst_capacity() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 600_000, 5));
        advance_time(500_000);
        let charge = consume_global(ALICE, 10_000).unwrap();
        advance_time(500_000);
        assert_ok!(Subscription::correct(charge, Usage::Transaction(0)));
        assert_eq!(global_bucket(ALICE).available, 10_000);
    });
}

#[test]
fn extra_usage_never_makes_bucket_negative() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(100);
        let charge = consume_global(ALICE, 100).unwrap();
        assert_ok!(Subscription::correct(charge, Usage::Transaction(10_000)));
        assert_eq!(global_bucket(ALICE).available, 0);
    });
}

#[test]
fn local_correction_updates_global_and_local_together() {
    new_test_ext().execute_with(|| {
        set_local_limit(7, 4);
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(100);
        let charge = consume_local(ALICE, 7, 300).unwrap();
        assert_eq!(charge.generation(), 1);
        assert_eq!(charge.charged(), Usage::Transaction(300));
        assert_ok!(Subscription::correct(charge, Usage::Transaction(100)));
        assert_eq!(global_bucket(ALICE).available, 500 - 100);
        let ResourceState::Rate(local) = local_bucket(ALICE, 7).state;
        assert_eq!(local.available, 400 - 100);
    });
}

#[test]
fn local_correction_for_replaced_generation_is_rejected_atomically() {
    new_test_ext().execute_with(|| {
        set_local_limit(7, 4);
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(100);
        let charge = consume_local(ALICE, 7, 300).unwrap();
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(100);
        assert_ok!(consume_local(ALICE, 7, 100));
        let global = global_bucket(ALICE);
        let local = local_bucket(ALICE, 7);
        assert_noop!(
            Subscription::correct(charge, Usage::Transaction(0)),
            Error::<Runtime>::WrongGeneration
        );
        assert_eq!(global_bucket(ALICE), global);
        assert_eq!(local_bucket(ALICE, 7), local);
    });
}

#[test]
fn correction_after_revocation_is_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(100);
        let charge = consume_global(ALICE, 100).unwrap();
        assert_ok!(Subscription::revoke(&ALICE));
        assert_noop!(
            Subscription::correct(charge, Usage::Transaction(0)),
            Error::<Runtime>::NoSubscription
        );
    });
}

#[test]
fn charge_records_accounting_source() {
    new_test_ext().execute_with(|| {
        assert_ok!(allocate(ALICE, 60_000, 5));
        advance_time(100);
        let charge = consume_global(ALICE, 100).unwrap();
        assert_eq!(charge.0.source, ChargeSource::Global);
    });
}

// ---------------------------------------------------------------------------
// Limit-type independence and allocator independence
// ---------------------------------------------------------------------------

#[test]
fn limit_and_usage_are_distinct_types() {
    assert_ne!(
        core::any::TypeId::of::<Limit>(),
        core::any::TypeId::of::<Usage>()
    );
    assert_eq!(Limit::Transaction(5).resource(), Resource::Transaction);
    assert_eq!(Usage::Transaction(5).resource(), Resource::Transaction);
}

#[test]
fn identical_allocations_give_identical_state_regardless_of_caller() {
    let state = |owner: u64| {
        let mut ext = new_test_ext();
        ext.execute_with(|| {
            assert_ok!(allocate(owner, 5_000, 5));
            advance_time(100);
            assert_ok!(consume_global(owner, 100));
            (
                Subscriptions::<Runtime>::get(owner),
                global_bucket(owner),
                TotalAllocatedTransactionRate::<Runtime>::get(),
            )
        })
    };
    assert_eq!(state(ALICE), state(ALICE));
}
