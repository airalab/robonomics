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
//! Tests for pallet-robonomics-subscription-governance

use crate::{self as pallet_governance, *};
use pallet_robonomics_subscription::{
    Amount, LastGeneration, Limit, Subscription as SubscriptionState, Subscriptions,
    TotalAllocatedTransactionRate,
};
use polkadot_sdk::frame_support::{
    self, assert_noop, assert_ok, derive_impl, parameter_types,
    traits::{ConstU32, Time},
    BoundedVec,
};
use polkadot_sdk::frame_system::{self, EnsureRoot, RawOrigin};
use polkadot_sdk::sp_io;
use polkadot_sdk::sp_runtime::{BuildStorage, DispatchError, DispatchResult, Weight};
use robonomics_subscription_primitives::{
    Resource, ResourceAllocation, ResourceLimit, SubscriptionAllocation, SubscriptionAllocator,
};
use std::cell::RefCell;

type Block = frame_system::mocking::MockBlock<Runtime>;

frame_support::construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Subscription: pallet_robonomics_subscription,
        Governance: pallet_governance,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type Block = Block;
    type AccountData = ();
}

thread_local! {
    static NOW: RefCell<u64> = const { RefCell::new(0) };
    static CALLS: RefCell<Vec<Call>> = const { RefCell::new(Vec::new()) };
}

/// Runtime clock controlled by the tests.
pub struct MockTime;

impl Time for MockTime {
    type Moment = u64;

    fn now() -> u64 {
        NOW.with(|now| *now.borrow())
    }
}

/// No local limits: they are irrelevant to allocators.
pub struct NoLimits;

impl ResourceLimit for NoLimits {
    type LocalId = u32;
    type ResourceId = Resource;
    type Limit = Limit;

    fn limit(_local: &u32, _resource: Resource) -> Option<Limit> {
        None
    }
}

parameter_types! {
    pub const BurstPeriod: u64 = 2_000;
    pub const MaxRate: Amount = 100;
    pub const BucketSize: u64 = 1_000;
    pub const MaxPerBucket: u32 = 8;
    pub const MaxPerBlock: u32 = 8;
}

impl pallet_robonomics_subscription::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Time = MockTime;
    type LocalId = u32;
    type ResourceLimits = NoLimits;
    type TransactionBurstPeriod = BurstPeriod;
    type MaxSubscriptionTransactionRate = MaxRate;
    type MaxResources = ConstU32<3>;
    type ExpirationBucketSize = BucketSize;
    type MaxExpirationsPerBucket = MaxPerBucket;
    type MaxExpirationsPerBlock = MaxPerBlock;
    type WeightInfo = ();
}

type Allocation = SubscriptionAllocation<Resource, Limit, u64, ConstU32<3>>;

/// Call observed by [`Recorder`].
#[derive(Clone, PartialEq, Eq, Debug)]
enum Call {
    Allocate(u64, Allocation),
    Revoke(u64),
}

/// Allocator that records every call and delegates to the subscription
/// pallet, proving the governance pallet reaches it only through the
/// `SubscriptionAllocator` interface.
pub struct Recorder;

impl SubscriptionAllocator<u64> for Recorder {
    type ResourceId = Resource;
    type Limit = Limit;
    type Moment = u64;
    type MaxResources = ConstU32<3>;

    fn allocate(owner: &u64, allocation: Allocation) -> DispatchResult {
        CALLS.with(|calls| {
            calls
                .borrow_mut()
                .push(Call::Allocate(*owner, allocation.clone()))
        });
        <Subscription as SubscriptionAllocator<u64>>::allocate(owner, allocation)
    }

    fn revoke(owner: &u64) -> DispatchResult {
        CALLS.with(|calls| calls.borrow_mut().push(Call::Revoke(*owner)));
        <Subscription as SubscriptionAllocator<u64>>::revoke(owner)
    }

    fn is_active(owner: &u64) -> bool {
        <Subscription as SubscriptionAllocator<u64>>::is_active(owner)
    }

    fn allocate_weight(resources: u32) -> Weight {
        <Subscription as SubscriptionAllocator<u64>>::allocate_weight(resources)
    }
}

impl pallet_governance::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AllocateOrigin = EnsureRoot<u64>;
    type Allocator = Recorder;
    type WeightInfo = ();
}

const ALICE: u64 = 1;
const BOB: u64 = 2;

fn new_test_ext() -> sp_io::TestExternalities {
    NOW.with(|now| *now.borrow_mut() = 10_000);
    CALLS.with(|calls| calls.borrow_mut().clear());
    let storage = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(storage);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn allocation_of(valid_for: u64, resources: Vec<(Resource, Limit)>) -> Allocation {
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

fn transaction(valid_for: u64, rate: Amount) -> Allocation {
    allocation_of(
        valid_for,
        vec![(Resource::Transaction, Limit::Transaction(rate))],
    )
}

fn root() -> RuntimeOrigin {
    RawOrigin::Root.into()
}

#[test]
fn authorized_origin_can_allocate() {
    new_test_ext().execute_with(|| {
        assert_ok!(Governance::allocate(root(), ALICE, transaction(5_000, 10)));

        assert_eq!(
            Subscriptions::<Runtime>::get(ALICE),
            Some(SubscriptionState {
                generation: 1,
                started_at: 10_000,
                expires_at: 15_000,
            })
        );
        System::assert_has_event(Event::<Runtime>::Allocated { beneficiary: ALICE }.into());
    });
}

#[test]
fn unauthorized_origin_cannot_allocate() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Governance::allocate(RawOrigin::Signed(BOB).into(), ALICE, transaction(5_000, 10)),
            DispatchError::BadOrigin
        );
        assert_noop!(
            Governance::allocate(RawOrigin::None.into(), ALICE, transaction(5_000, 10)),
            DispatchError::BadOrigin
        );
        assert!(Subscriptions::<Runtime>::get(ALICE).is_none());
        assert!(CALLS.with(|calls| calls.borrow().is_empty()));
    });
}

#[test]
fn allocation_replaces_previous_generation() {
    new_test_ext().execute_with(|| {
        assert_ok!(Governance::allocate(root(), ALICE, transaction(5_000, 10)));
        NOW.with(|now| *now.borrow_mut() += 1_000);
        assert_ok!(Governance::allocate(root(), ALICE, transaction(9_000, 20)));

        assert_eq!(
            Subscriptions::<Runtime>::get(ALICE),
            Some(SubscriptionState {
                generation: 2,
                started_at: 11_000,
                expires_at: 20_000,
            })
        );
        assert_eq!(LastGeneration::<Runtime>::get(ALICE), 2);
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 20);
    });
}

#[test]
fn allocation_is_forwarded_verbatim() {
    new_test_ext().execute_with(|| {
        let empty = allocation_of(7_000, vec![]);
        let rate = transaction(3_000, 42);
        assert_ok!(Governance::allocate(root(), ALICE, empty.clone()));
        assert_ok!(Governance::allocate(root(), BOB, rate.clone()));

        assert_eq!(
            CALLS.with(|calls| calls.borrow().clone()),
            vec![Call::Allocate(ALICE, empty), Call::Allocate(BOB, rate)]
        );
    });
}

#[test]
fn invalid_allocation_is_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Governance::allocate(root(), ALICE, transaction(0, 10)),
            pallet_robonomics_subscription::Error::<Runtime>::ZeroDuration
        );
        assert_noop!(
            Governance::allocate(root(), ALICE, transaction(5_000, 0)),
            pallet_robonomics_subscription::Error::<Runtime>::InvalidResourceAllocation
        );
        assert_noop!(
            Governance::allocate(
                root(),
                ALICE,
                allocation_of(
                    5_000,
                    vec![
                        (Resource::Transaction, Limit::Transaction(1)),
                        (Resource::Transaction, Limit::Transaction(2)),
                    ]
                )
            ),
            pallet_robonomics_subscription::Error::<Runtime>::DuplicateResource
        );
        assert_noop!(
            Governance::allocate(
                root(),
                ALICE,
                allocation_of(5_000, vec![(Resource::Storage, Limit::Transaction(1))])
            ),
            pallet_robonomics_subscription::Error::<Runtime>::UnknownResource
        );
        assert!(Subscriptions::<Runtime>::get(ALICE).is_none());
    });
}

#[test]
fn capacity_failure_is_propagated_and_keeps_state() {
    new_test_ext().execute_with(|| {
        assert_ok!(Governance::allocate(root(), ALICE, transaction(5_000, 60)));
        let events = System::events().len();

        assert_noop!(
            Governance::allocate(root(), BOB, transaction(5_000, 41)),
            pallet_robonomics_subscription::Error::<Runtime>::GlobalRateCapacityExceeded
        );
        assert!(Subscriptions::<Runtime>::get(BOB).is_none());
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 60);
        assert_eq!(System::events().len(), events);
    });
}

#[test]
fn revoke_is_governed_and_forwarded() {
    new_test_ext().execute_with(|| {
        assert_ok!(Governance::allocate(root(), ALICE, transaction(5_000, 10)));

        assert_noop!(
            Governance::revoke(RawOrigin::Signed(ALICE).into(), ALICE),
            DispatchError::BadOrigin
        );
        assert_ok!(Governance::revoke(root(), ALICE));

        assert!(Subscriptions::<Runtime>::get(ALICE).is_none());
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 0);
        assert_eq!(
            CALLS.with(|calls| calls.borrow().last().cloned()),
            Some(Call::Revoke(ALICE))
        );
        System::assert_has_event(Event::<Runtime>::Revoked { beneficiary: ALICE }.into());
    });
}

#[test]
fn revoke_without_subscription_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Governance::revoke(root(), ALICE),
            pallet_robonomics_subscription::Error::<Runtime>::NoSubscription
        );
    });
}
