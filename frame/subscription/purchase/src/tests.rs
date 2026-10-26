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
//! Tests for pallet-robonomics-subscription-purchase

use crate::{self as pallet_purchase, *};
use pallet_robonomics_subscription::{
    Amount, GlobalResourceState, LastGeneration, Limit, ResourceAllocations,
    Subscription as SubscriptionState, Subscriptions, TotalAllocatedTransactionRate,
};
use polkadot_sdk::frame_support::{
    self, assert_noop, assert_ok, derive_impl, parameter_types,
    traits::{ConstU32, Time},
    BoundedVec,
};
use polkadot_sdk::frame_system::{self, EnsureRoot, RawOrigin};
use polkadot_sdk::sp_io;
use polkadot_sdk::sp_runtime::{BuildStorage, DispatchError};
use robonomics_subscription_primitives::{
    Resource, ResourceAllocation, ResourceLimit, SubscriptionAllocation,
};
use std::cell::RefCell;

type Block = frame_system::mocking::MockBlock<Runtime>;

frame_support::construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Subscription: pallet_robonomics_subscription,
        Governance: pallet_robonomics_subscription_governance,
        Purchase: pallet_purchase,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type Block = Block;
    type AccountData = ();
}

thread_local! {
    static NOW: RefCell<u64> = const { RefCell::new(0) };
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

impl pallet_robonomics_subscription_governance::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AllocateOrigin = EnsureRoot<u64>;
    type Allocator = Subscription;
    type WeightInfo = ();
}

impl pallet_purchase::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ManagementOrigin = EnsureRoot<u64>;
    type Allocator = Subscription;
    type Balance = u64;
    type DepositProduct = ConstU32<0>;
    type WeightInfo = ();
}

const ALICE: u64 = 1;
const BOB: u64 = 2;
const CAROL: u64 = 3;

type Allocation = SubscriptionAllocation<Resource, Limit, u64, ConstU32<3>>;

fn new_test_ext() -> sp_io::TestExternalities {
    NOW.with(|now| *now.borrow_mut() = 10_000);
    let storage = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(storage);
    ext.execute_with(|| {
        System::set_block_number(1);
    });
    ext
}

fn resources(rate: Amount) -> ResourcesOf<Runtime> {
    BoundedVec::truncate_from(vec![ResourceAllocation {
        resource: Resource::Transaction,
        limit: Limit::Transaction(rate),
    }])
}

fn root() -> RuntimeOrigin {
    RawOrigin::Root.into()
}

fn signed(account: u64) -> RuntimeOrigin {
    RawOrigin::Signed(account).into()
}

/// Create a product `(price, valid_for, rate)` and return its id.
fn create_product(price: u64, valid_for: u64, rate: Amount) -> ProductId {
    let id = NextProductId::<Runtime>::get();
    assert_ok!(Purchase::create_product(
        root(),
        price,
        valid_for,
        resources(rate)
    ));
    id
}

#[test]
fn management_origin_manages_products() {
    new_test_ext().execute_with(|| {
        let id = create_product(100, 5_000, 5);
        assert_eq!(
            Products::<Runtime>::get(id),
            Some(SubscriptionProduct {
                price: 100,
                valid_for: 5_000,
                resources: resources(5),
                enabled: true,
            })
        );
        System::assert_has_event(Event::<Runtime>::ProductCreated { product_id: id }.into());

        assert_ok!(Purchase::update_product(
            root(),
            id,
            200,
            6_000,
            resources(7)
        ));
        assert_eq!(
            Products::<Runtime>::get(id),
            Some(SubscriptionProduct {
                price: 200,
                valid_for: 6_000,
                resources: resources(7),
                enabled: true,
            })
        );

        assert_ok!(Purchase::disable_product(root(), id));
        assert!(!Products::<Runtime>::get(id).unwrap().enabled);

        let next = create_product(1, 1, 1);
        assert_eq!(next, id + 1);
    });
}

#[test]
fn unauthorized_origin_cannot_manage_products() {
    new_test_ext().execute_with(|| {
        let id = create_product(100, 5_000, 5);
        assert_noop!(
            Purchase::create_product(signed(ALICE), 1, 1, resources(1)),
            DispatchError::BadOrigin
        );
        assert_noop!(
            Purchase::update_product(signed(ALICE), id, 1, 1, resources(1)),
            DispatchError::BadOrigin
        );
        assert_noop!(
            Purchase::disable_product(signed(ALICE), id),
            DispatchError::BadOrigin
        );
    });
}

/// Product 0 costs 100, lasts 5_000 and grants a Transaction rate of 5.
fn create_deposit_product() {
    assert_eq!(create_product(100, 5_000, 5), 0);
}

fn subscription_of(account: u64) -> Option<SubscriptionState<u64>> {
    Subscriptions::<Runtime>::get(account)
}

#[test]
fn unknown_product_cannot_be_updated_or_disabled() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Purchase::update_product(root(), 0, 1, 1, resources(1)),
            Error::<Runtime>::UnknownProduct
        );
        assert_noop!(
            Purchase::disable_product(root(), 0),
            Error::<Runtime>::UnknownProduct
        );
    });
}

#[test]
fn deposit_covering_the_price_allocates_the_product() {
    new_test_ext().execute_with(|| {
        create_deposit_product();
        assert_ok!(Purchase::purchase_with_deposit(&ALICE, 100));

        assert_eq!(
            subscription_of(ALICE),
            Some(SubscriptionState {
                generation: 1,
                started_at: 10_000,
                expires_at: 15_000,
            })
        );
        assert_eq!(
            ResourceAllocations::<Runtime>::get(ALICE, Resource::Transaction),
            Some(Limit::Transaction(5))
        );
        System::assert_has_event(
            Event::<Runtime>::Purchased {
                beneficiary: ALICE,
                product_id: 0,
                paid: 100,
            }
            .into(),
        );
    });
}

#[test]
fn deposit_above_the_price_allocates_the_same_product() {
    new_test_ext().execute_with(|| {
        create_deposit_product();
        assert_ok!(Purchase::purchase_with_deposit(&ALICE, 1_000));
        assert_ok!(Purchase::purchase_with_deposit(&BOB, 100));

        assert_eq!(subscription_of(ALICE), subscription_of(BOB));
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 10);
    });
}

#[test]
fn deposit_below_the_price_is_rejected() {
    new_test_ext().execute_with(|| {
        create_deposit_product();
        assert_noop!(
            Purchase::purchase_with_deposit(&ALICE, 99),
            Error::<Runtime>::DepositBelowPrice
        );
        assert!(subscription_of(ALICE).is_none());
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 0);
    });
}

#[test]
fn missing_or_disabled_product_rejects_deposits() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Purchase::purchase_with_deposit(&ALICE, 100),
            Error::<Runtime>::UnknownProduct
        );
        create_deposit_product();
        assert_ok!(Purchase::disable_product(root(), 0));
        assert_noop!(
            Purchase::purchase_with_deposit(&ALICE, 100),
            Error::<Runtime>::ProductDisabled
        );
        assert!(subscription_of(ALICE).is_none());
    });
}

#[test]
fn allocation_failure_rejects_the_deposit() {
    new_test_ext().execute_with(|| {
        assert_eq!(create_product(100, 0, 5), 0);

        assert_noop!(
            Purchase::purchase_with_deposit(&ALICE, 100),
            pallet_robonomics_subscription::Error::<Runtime>::ZeroDuration
        );
        assert!(subscription_of(ALICE).is_none());
    });
}

#[test]
fn capacity_failure_rejects_the_deposit() {
    new_test_ext().execute_with(|| {
        assert_eq!(create_product(100, 5_000, 80), 0);
        assert_ok!(Purchase::purchase_with_deposit(&ALICE, 100));
        let events = System::events().len();

        assert_noop!(
            Purchase::purchase_with_deposit(&BOB, 100),
            pallet_robonomics_subscription::Error::<Runtime>::GlobalRateCapacityExceeded
        );
        assert!(subscription_of(BOB).is_none());
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 80);
        assert_eq!(System::events().len(), events);
    });
}

#[test]
fn purchase_replaces_previous_generation() {
    new_test_ext().execute_with(|| {
        create_deposit_product();
        assert_ok!(Purchase::purchase_with_deposit(&ALICE, 100));
        assert_ok!(Purchase::set_replacement_consent(signed(ALICE), true));
        NOW.with(|now| *now.borrow_mut() += 1_000);
        assert_ok!(Purchase::purchase_with_deposit(&ALICE, 100));

        assert_eq!(
            subscription_of(ALICE),
            Some(SubscriptionState {
                generation: 2,
                started_at: 11_000,
                expires_at: 16_000,
            })
        );
        assert_eq!(LastGeneration::<Runtime>::get(ALICE), 2);
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 5);
    });
}

#[test]
fn product_update_does_not_modify_existing_subscriptions() {
    new_test_ext().execute_with(|| {
        create_deposit_product();
        assert_ok!(Purchase::purchase_with_deposit(&ALICE, 100));
        let subscription = subscription_of(ALICE);

        assert_ok!(Purchase::update_product(
            root(),
            0,
            500,
            50_000,
            resources(50)
        ));
        assert_ok!(Purchase::disable_product(root(), 0));

        assert_eq!(subscription_of(ALICE), subscription);
        assert_eq!(
            ResourceAllocations::<Runtime>::get(ALICE, Resource::Transaction),
            Some(Limit::Transaction(5))
        );
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 5);
    });
}

#[test]
fn updated_product_applies_to_future_purchases() {
    new_test_ext().execute_with(|| {
        create_deposit_product();
        assert_ok!(Purchase::update_product(
            root(),
            0,
            250,
            8_000,
            resources(9)
        ));

        assert_noop!(
            Purchase::purchase_with_deposit(&ALICE, 100),
            Error::<Runtime>::DepositBelowPrice
        );
        assert_ok!(Purchase::purchase_with_deposit(&ALICE, 250));
        assert_eq!(subscription_of(ALICE).map(|s| s.expires_at), Some(18_000));
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 9);
    });
}

#[test]
fn purchased_allocation_matches_product_definition() {
    new_test_ext().execute_with(|| {
        create_deposit_product();
        let expected: Allocation = SubscriptionAllocation {
            valid_for: 5_000,
            resources: resources(5),
        };
        assert_eq!(Products::<Runtime>::get(0).unwrap().allocation(), expected);

        assert_ok!(Purchase::purchase_with_deposit(&ALICE, 100));
        assert_eq!(
            ResourceAllocations::<Runtime>::iter_prefix(ALICE).collect::<Vec<_>>(),
            vec![(Resource::Transaction, Limit::Transaction(5))]
        );
    });
}

#[test]
fn product_management_does_not_reserve_capacity() {
    new_test_ext().execute_with(|| {
        // The product alone exceeds the network capacity of 100.
        create_product(100, 5_000, 1_000);
        assert_ok!(Purchase::update_product(
            root(),
            0,
            100,
            5_000,
            resources(90)
        ));

        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 0);
        assert_ok!(Governance::allocate(
            root(),
            CAROL,
            SubscriptionAllocation {
                valid_for: 5_000,
                resources: resources(100),
            }
        ));
    });
}

/// Governance and Purchase must create equivalent subscription state from
/// identical normalized allocations.
#[test]
fn allocators_produce_equivalent_subscriptions() {
    new_test_ext().execute_with(|| {
        create_deposit_product();
        assert_ok!(Purchase::purchase_with_deposit(&ALICE, 100));
        assert_ok!(Governance::allocate(
            root(),
            BOB,
            SubscriptionAllocation {
                valid_for: 5_000,
                resources: resources(5),
            }
        ));

        assert_eq!(subscription_of(ALICE), subscription_of(BOB));
        assert_eq!(
            LastGeneration::<Runtime>::get(ALICE),
            LastGeneration::<Runtime>::get(BOB)
        );
        assert_eq!(
            ResourceAllocations::<Runtime>::iter_prefix(ALICE).collect::<Vec<_>>(),
            ResourceAllocations::<Runtime>::iter_prefix(BOB).collect::<Vec<_>>()
        );
        assert_eq!(
            GlobalResourceState::<Runtime>::get(ALICE, Resource::Transaction),
            GlobalResourceState::<Runtime>::get(BOB, Resource::Transaction)
        );
        assert_eq!(TotalAllocatedTransactionRate::<Runtime>::get(), 10);
    });
}

/// Both allocators fail the same way when the network cannot admit a rate.
#[test]
fn allocators_share_capacity_admission() {
    new_test_ext().execute_with(|| {
        assert_eq!(create_product(100, 5_000, 60), 0);
        assert_ok!(Purchase::purchase_with_deposit(&ALICE, 100));

        let purchase = Purchase::purchase_with_deposit(&BOB, 100);
        let governance = Governance::allocate(
            root(),
            BOB,
            SubscriptionAllocation {
                valid_for: 5_000,
                resources: resources(60),
            },
        );
        assert_eq!(purchase, governance);
        assert_eq!(
            purchase,
            Err(
                pallet_robonomics_subscription::Error::<Runtime>::GlobalRateCapacityExceeded.into()
            )
        );
    });
}

#[test]
fn third_party_purchase_cannot_replace_an_active_subscription() {
    new_test_ext().execute_with(|| {
        create_deposit_product();
        assert_ok!(Purchase::purchase_with_deposit(&ALICE, 100));
        let subscription = subscription_of(ALICE);
        let events = System::events().len();

        assert_noop!(
            Purchase::purchase_with_deposit(&ALICE, 100),
            Error::<Runtime>::ReplacementNotConsented
        );
        assert_eq!(subscription_of(ALICE), subscription);
        assert_eq!(System::events().len(), events);
    });
}

#[test]
fn expired_subscription_is_replaced_without_consent() {
    new_test_ext().execute_with(|| {
        create_deposit_product();
        assert_ok!(Purchase::purchase_with_deposit(&ALICE, 100));
        NOW.with(|now| *now.borrow_mut() += 5_000);

        assert_ok!(Purchase::purchase_with_deposit(&ALICE, 100));
        assert_eq!(LastGeneration::<Runtime>::get(ALICE), 2);
    });
}

#[test]
fn consent_can_be_withdrawn() {
    new_test_ext().execute_with(|| {
        create_deposit_product();
        assert_ok!(Purchase::purchase_with_deposit(&ALICE, 100));

        assert_ok!(Purchase::set_replacement_consent(signed(ALICE), true));
        System::assert_last_event(
            Event::<Runtime>::ReplacementConsentSet {
                account: ALICE,
                allowed: true,
            }
            .into(),
        );
        assert_ok!(Purchase::set_replacement_consent(signed(ALICE), false));
        assert!(!ReplacementConsent::<Runtime>::contains_key(ALICE));
        assert_noop!(
            Purchase::purchase_with_deposit(&ALICE, 100),
            Error::<Runtime>::ReplacementNotConsented
        );
    });
}

#[test]
fn consent_requires_a_signed_origin() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Purchase::set_replacement_consent(root(), true),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn deposit_weight_includes_the_allocation() {
    let allocation = <Subscription as SubscriptionAllocator<u64>>::allocate_weight(3);
    let weight = Purchase::purchase_with_deposit_weight();
    assert!(weight.all_gte(allocation));
    assert!(weight.proof_size() > allocation.proof_size());
}
