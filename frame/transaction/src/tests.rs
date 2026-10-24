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
//! Tests for pallet-robonomics-transaction

use crate::{self as pallet_transaction, priority, ChargeTransaction, Payment};
use pallet_robonomics_cps::{Capability, GrantMode, NodeId, ScopeId};
use pallet_robonomics_subscription::{Limit, Usage};
use parity_scale_codec::Encode;
use polkadot_sdk::{
    frame_support::{
        self, assert_ok, derive_impl,
        dispatch::{DispatchClass, DispatchInfo, PostDispatchInfo},
        pallet_prelude::Pays,
        parameter_types,
        traits::{ConstU64, ConstU8, Get},
        weights::{IdentityFee, Weight},
        BoundedVec,
    },
    frame_system::{self, limits::BlockWeights, RawOrigin},
    pallet_balances,
    pallet_transaction_payment::{self, FungibleAdapter, Multiplier},
    sp_io,
    sp_runtime::{
        self,
        testing::TestSignature,
        traits::{Convert, DispatchTransaction, TransactionExtension},
        transaction_validity::{InvalidTransaction, TransactionSource, TransactionValidityError},
        BuildStorage, FixedPointNumber,
    },
};
use robonomics_subscription_primitives::{
    AuthorizationContext, Resource, ResourceAllocation, ResourceLimit, SubscriptionAllocation,
    SubscriptionAllocator,
};
use std::{cell::RefCell, collections::BTreeMap};

type Block = frame_system::mocking::MockBlock<Runtime>;
type Ext = ChargeTransaction<Runtime>;
type Context = AuthorizationContext<u64, NodeId>;

frame_support::construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Balances: pallet_balances,
        TransactionPayment: pallet_transaction_payment,
        Cps: pallet_robonomics_cps,
        Subscription: pallet_robonomics_subscription,
        Transaction: pallet_transaction,
    }
);

parameter_types! {
    pub Weights: BlockWeights = BlockWeights::builder()
        .base_block(Weight::zero())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = Weight::from_parts(BASE_EXTRINSIC, 0);
            weights.max_total = Some(Weight::from_parts(1_000_000_000, 1_000_000));
        })
        .build_or_panic();
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type Block = Block;
    type AccountData = pallet_balances::AccountData<u64>;
    type BlockWeights = Weights;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Runtime {
    type AccountStore = System;
    type ExistentialDeposit = ConstU64<1>;
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = FungibleAdapter<Balances, ()>;
    type OperationalFeeMultiplier = ConstU8<5>;
    type WeightToFee = IdentityFee<u64>;
    type LengthToFee = IdentityFee<u64>;
    type FeeMultiplierUpdate = ();
    type WeightInfo = ();
}

#[cfg(feature = "runtime-benchmarks")]
pub struct BenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_robonomics_cps::BenchmarkHelper<u64, TestSignature> for BenchmarkHelper {
    fn identity_with_signature(payload: &[u8]) -> (u64, TestSignature) {
        (0x1d, TestSignature(0x1d, payload.to_vec()))
    }
}

impl pallet_robonomics_cps::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_robonomics_cps::weights::TestWeightInfo;
    type Signature = TestSignature;
    type MaxIdentityPermitTtl = ConstU64<100>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = BenchmarkHelper;
}

thread_local! {
    static NOW: RefCell<u64> = const { RefCell::new(0) };
    static LIMITS: RefCell<BTreeMap<u64, u128>> = const { RefCell::new(BTreeMap::new()) };
}

/// Runtime clock controlled by the tests.
pub struct MockTime;

impl frame_support::traits::Time for MockTime {
    type Moment = u64;

    fn now() -> u64 {
        NOW.with(|now| *now.borrow())
    }
}

/// Local Transaction rates controlled by the tests.
pub struct MockLimits;

impl ResourceLimit for MockLimits {
    type LocalId = ScopeId;
    type ResourceId = Resource;
    type Limit = Limit;

    fn limit(local: &ScopeId, resource: Resource) -> Option<Limit> {
        match resource {
            Resource::Transaction => LIMITS
                .with(|limits| limits.borrow().get(&local.0).copied())
                .map(Limit::Transaction),
            _ => None,
        }
    }
}

parameter_types! {
    pub const BurstPeriod: u64 = 10_000;
    pub const MaxRate: u128 = 1_000_000;
    pub const BucketSize: u64 = 1_000;
}

impl pallet_robonomics_subscription::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Time = MockTime;
    type LocalId = ScopeId;
    type ResourceLimits = MockLimits;
    type TransactionBurstPeriod = BurstPeriod;
    type MaxSubscriptionTransactionRate = MaxRate;
    type MaxResources = frame_support::traits::ConstU32<3>;
    type ExpirationBucketSize = BucketSize;
    type MaxExpirationsPerBucket = frame_support::traits::ConstU32<8>;
    type MaxExpirationsPerBlock = frame_support::traits::ConstU32<8>;
    type WeightInfo = ();
}

/// Baseline cost to subscription usage.
pub struct ToUsage;

impl Convert<u64, Usage> for ToUsage {
    fn convert(cost: u64) -> Usage {
        Usage::Transaction(cost as u128)
    }
}

impl pallet_transaction::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ContextId = Context;
    type LocalId = ScopeId;
    type Authorization = Cps;
    type Accounting = Subscription;
    type ToUsage = ToUsage;
    type WeightInfo = ();
}

const BASE_EXTRINSIC: u64 = 100;
const OWNER: u64 = 1;
const ACTOR: u64 = 2;
const STRANGER: u64 = 3;
const IDENTITY: u64 = 9;
const LEN: usize = 10;
const WEIGHT: u64 = 500;
/// `BASE_EXTRINSIC + LEN + WEIGHT`.
const COST: u64 = 610;
const START: u64 = 10_000;

fn new_test_ext() -> sp_io::TestExternalities {
    NOW.with(|now| *now.borrow_mut() = START);
    LIMITS.with(|limits| limits.borrow_mut().clear());
    let mut storage = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();
    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![
            (OWNER, 1_000_000),
            (ACTOR, 1_000_000),
            (STRANGER, 1_000_000),
        ],
        ..Default::default()
    }
    .assimilate_storage(&mut storage)
    .unwrap();
    let mut ext = sp_io::TestExternalities::new(storage);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn advance_time(by: u64) {
    NOW.with(|now| *now.borrow_mut() += by);
}

fn set_local_rate(scope: u64, rate: u128) {
    LIMITS.with(|limits| limits.borrow_mut().insert(scope, rate));
}

fn call() -> RuntimeCall {
    RuntimeCall::System(frame_system::Call::remark { remark: vec![] })
}

fn info(weight: u64, class: DispatchClass) -> DispatchInfo {
    DispatchInfo {
        call_weight: Weight::from_parts(weight, 0),
        extension_weight: Weight::zero(),
        class,
        pays_fee: Pays::Yes,
    }
}

fn normal() -> DispatchInfo {
    info(WEIGHT, DispatchClass::Normal)
}

/// Subscribe `owner` to a global Transaction rate and return the generation.
fn subscribe(owner: u64, rate: u128) -> u64 {
    assert_ok!(Subscription::allocate(
        &owner,
        SubscriptionAllocation {
            valid_for: 1_000_000,
            resources: BoundedVec::truncate_from(vec![ResourceAllocation {
                resource: Resource::Transaction,
                limit: Limit::Transaction(rate),
            }]),
        }
    ));
    generation(owner)
}

fn generation(owner: u64) -> u64 {
    pallet_robonomics_subscription::Subscriptions::<Runtime>::get(owner)
        .unwrap()
        .generation
}

fn global_available(owner: u64) -> u128 {
    let pallet_robonomics_subscription::ResourceState::Rate(bucket) =
        pallet_robonomics_subscription::GlobalResourceState::<Runtime>::get(
            owner,
            Resource::Transaction,
        )
        .unwrap()
        .state;
    bucket.available
}

fn local_available(owner: u64, scope: u64) -> Option<u128> {
    pallet_robonomics_subscription::LocalResourceState::<Runtime>::get((
        owner,
        ScopeId(scope),
        Resource::Transaction,
    ))
    .map(|state| {
        let pallet_robonomics_subscription::ResourceState::Rate(bucket) = state.state;
        bucket.available
    })
}

/// `OWNER` roots Scope 0 at node 0 and delegates the Transaction capability
/// at it to `ACTOR`.
fn setup_node() {
    assert_ok!(Cps::create_node(
        RuntimeOrigin::signed(OWNER),
        None,
        None,
        None,
        None
    ));
    assert_ok!(Cps::grant_access(
        RuntimeOrigin::signed(OWNER),
        NodeId(0),
        ACTOR,
        Capability::Transaction,
        GrantMode::Node,
    ));
}

fn invalid(error: InvalidTransaction) -> Option<TransactionValidityError> {
    Some(error.into())
}

fn validate(
    ext: &Ext,
    who: u64,
    info: &DispatchInfo,
) -> Result<
    (
        sp_runtime::transaction_validity::ValidTransaction,
        RuntimeOrigin,
    ),
    TransactionValidityError,
> {
    ext.validate_only(
        RawOrigin::Signed(who).into(),
        &call(),
        info,
        LEN,
        TransactionSource::External,
        0,
    )
    .map(|(valid, _, origin)| (valid, origin))
}

fn prepare(
    ext: Ext,
    who: u64,
    info: &DispatchInfo,
) -> Result<(crate::Pre<Runtime>, RuntimeOrigin), TransactionValidityError> {
    ext.validate_and_prepare(RawOrigin::Signed(who).into(), &call(), info, LEN, 0)
}

fn post(pre: crate::Pre<Runtime>, info: &DispatchInfo, actual_weight: Option<u64>) {
    let mut post_info = PostDispatchInfo {
        actual_weight: actual_weight.map(|weight| Weight::from_parts(weight, 0)),
        pays_fee: Pays::Yes,
    };
    assert_ok!(Ext::post_dispatch(pre, info, &mut post_info, LEN, &Ok(())));
}

fn paid_events() -> Vec<pallet_transaction::Event<Runtime>> {
    System::events()
        .into_iter()
        .filter_map(|record| match record.event {
            RuntimeEvent::Transaction(event) => Some(event),
            _ => None,
        })
        .collect()
}

fn account_ext(owner: u64, generation: u64) -> Ext {
    Ext::subscription(AuthorizationContext::Account(owner), generation)
}

fn node_ext(node: u64, generation: u64) -> Ext {
    Ext::subscription(AuthorizationContext::Node(NodeId(node)), generation)
}

// ---------------------------------------------------------------------------
// Usage
// ---------------------------------------------------------------------------

#[test]
fn baseline_cost_is_base_plus_length_plus_weight() {
    new_test_ext().execute_with(|| {
        assert_eq!(Transaction::estimated_cost(LEN, &normal()), COST);
    });
}

#[test]
fn baseline_cost_ignores_congestion_but_fee_mode_does_not() {
    new_test_ext().execute_with(|| {
        let info = normal();
        let fee_at = |multiplier: u128| {
            pallet_transaction_payment::NextFeeMultiplier::<Runtime>::put(
                Multiplier::saturating_from_integer(multiplier),
            );
            pallet_transaction_payment::Pallet::<Runtime>::compute_fee(LEN as u32, &info, 0)
        };
        let calm_fee = fee_at(1);
        let calm_cost = Transaction::estimated_cost(LEN, &info);
        let busy_fee = fee_at(5);
        let busy_cost = Transaction::estimated_cost(LEN, &info);

        assert_eq!(calm_cost, busy_cost);
        assert_eq!(calm_fee, COST);
        assert!(busy_fee > calm_fee);
    });
}

#[test]
fn baseline_cost_ignores_tip() {
    new_test_ext().execute_with(|| {
        let subscription = Payment::<Context, u64, u64>::Subscription {
            context: AuthorizationContext::Account(OWNER),
            generation: 1,
        };
        // A Subscription payment has no tip at all.
        assert!(!matches!(subscription, Payment::Fee { .. }));
        let with_tip = |tip| {
            pallet_transaction_payment::Pallet::<Runtime>::compute_fee(LEN as u32, &normal(), tip)
        };
        assert_eq!(with_tip(1_000), COST + 1_000);
        assert_eq!(Transaction::estimated_cost(LEN, &normal()), COST);
    });
}

#[test]
fn transactions_that_pay_no_fee_cost_nothing() {
    new_test_ext().execute_with(|| {
        let mut free = normal();
        free.pays_fee = Pays::No;
        assert_eq!(Transaction::estimated_cost(LEN, &free), 0);
    });
}

#[test]
fn actual_cost_follows_the_post_dispatch_weight() {
    new_test_ext().execute_with(|| {
        let post_info = PostDispatchInfo {
            actual_weight: Some(Weight::from_parts(300, 0)),
            pays_fee: Pays::Yes,
        };
        assert_eq!(
            Transaction::actual_cost(LEN, &normal(), &post_info),
            BASE_EXTRINSIC + LEN as u64 + 300
        );
        let unchanged = PostDispatchInfo::default();
        assert_eq!(Transaction::actual_cost(LEN, &normal(), &unchanged), COST);
    });
}

// ---------------------------------------------------------------------------
// Account path
// ---------------------------------------------------------------------------

#[test]
fn owner_uses_own_subscription_with_global_accounting_only() {
    new_test_ext().execute_with(|| {
        let generation = subscribe(OWNER, 10);
        advance_time(100);
        let balance = Balances::free_balance(OWNER);

        let (pre, origin) = prepare(account_ext(OWNER, generation), OWNER, &normal()).unwrap();
        assert_eq!(global_available(OWNER), 1_000 - COST as u128);
        assert_eq!(local_available(OWNER, 0), None);
        // No fee is withdrawn.
        assert_eq!(Balances::free_balance(OWNER), balance);
        assert_eq!(frame_system::ensure_signed(origin).ok(), Some(OWNER));

        post(pre, &normal(), None);
        assert_eq!(global_available(OWNER), 1_000 - COST as u128);
        assert_eq!(
            paid_events(),
            vec![pallet_transaction::Event::SubscriptionTransactionPaid {
                who: OWNER,
                owner: OWNER,
                local_id: None,
                generation,
                usage: COST,
            }]
        );
    });
}

#[test]
fn account_context_of_another_actor_is_rejected() {
    new_test_ext().execute_with(|| {
        let generation = subscribe(OWNER, 10);
        advance_time(100);
        assert_eq!(
            prepare(account_ext(OWNER, generation), STRANGER, &normal()).err(),
            invalid(InvalidTransaction::BadSigner)
        );
        assert_eq!(global_available(OWNER), 0);
    });
}

#[test]
fn wrong_generation_is_rejected_and_never_uses_the_new_one() {
    new_test_ext().execute_with(|| {
        let old = subscribe(OWNER, 10);
        let new = subscribe(OWNER, 10);
        assert_eq!(new, old + 1);
        advance_time(100);
        assert_eq!(
            validate(&account_ext(OWNER, old), OWNER, &normal()).err(),
            invalid(InvalidTransaction::Custom(crate::GENERATION_MISMATCH))
        );
        assert_eq!(
            prepare(account_ext(OWNER, old), OWNER, &normal()).err(),
            invalid(InvalidTransaction::Custom(crate::GENERATION_MISMATCH))
        );
        assert_eq!(global_available(OWNER), 0);
        assert_ok!(prepare(account_ext(OWNER, new), OWNER, &normal()));
    });
}

#[test]
fn missing_or_expired_subscription_is_rejected() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            validate(&account_ext(OWNER, 1), OWNER, &normal()).err(),
            invalid(InvalidTransaction::Custom(crate::NO_ACTIVE_SUBSCRIPTION))
        );
        let generation = subscribe(OWNER, 10);
        advance_time(1_000_000);
        assert_eq!(
            validate(&account_ext(OWNER, generation), OWNER, &normal()).err(),
            invalid(InvalidTransaction::Custom(crate::NO_ACTIVE_SUBSCRIPTION))
        );
    });
}

#[test]
fn unsigned_origin_cannot_use_a_subscription() {
    new_test_ext().execute_with(|| {
        let generation = subscribe(OWNER, 10);
        let result = account_ext(OWNER, generation).validate_only(
            RawOrigin::None.into(),
            &call(),
            &normal(),
            LEN,
            TransactionSource::External,
            0,
        );
        assert_eq!(result.err(), invalid(InvalidTransaction::BadSigner));
    });
}

// ---------------------------------------------------------------------------
// Rate accounting
// ---------------------------------------------------------------------------

#[test]
fn insufficient_current_allowance_rejects_and_refills_with_time() {
    new_test_ext().execute_with(|| {
        let generation = subscribe(OWNER, 10);
        advance_time(100);
        assert_ok!(prepare(account_ext(OWNER, generation), OWNER, &normal()));
        // 390 left, 610 needed.
        assert_eq!(
            validate(&account_ext(OWNER, generation), OWNER, &normal()).err(),
            invalid(InvalidTransaction::Payment)
        );
        assert_eq!(
            prepare(account_ext(OWNER, generation), OWNER, &normal()).err(),
            invalid(InvalidTransaction::Payment)
        );
        advance_time(25);
        assert_ok!(prepare(account_ext(OWNER, generation), OWNER, &normal()));
    });
}

#[test]
fn validation_does_not_mutate_state() {
    new_test_ext().execute_with(|| {
        setup_node();
        set_local_rate(0, 10);
        let generation = subscribe(OWNER, 10);
        advance_time(100);
        let root = sp_io::storage::root(sp_runtime::StateVersion::V1);
        assert_ok!(validate(&node_ext(0, generation), ACTOR, &normal()));
        assert_ok!(validate(&account_ext(OWNER, generation), OWNER, &normal()));
        assert_eq!(root, sp_io::storage::root(sp_runtime::StateVersion::V1));
    });
}

#[test]
fn competing_transactions_cannot_overspend_in_pre_dispatch() {
    new_test_ext().execute_with(|| {
        let generation = subscribe(OWNER, 10);
        advance_time(100);
        // Both pass pool validation against the same allowance of 1_000.
        let first = account_ext(OWNER, generation);
        let second = account_ext(OWNER, generation);
        assert_ok!(validate(&first, OWNER, &normal()));
        assert_ok!(validate(&second, OWNER, &normal()));
        assert_ok!(prepare(first, OWNER, &normal()));
        assert_eq!(
            prepare(second, OWNER, &normal()).err(),
            invalid(InvalidTransaction::Payment)
        );
    });
}

#[test]
fn transaction_pallet_keeps_no_storage() {
    use frame_support::traits::StorageInfoTrait;

    new_test_ext().execute_with(|| {
        let generation = subscribe(OWNER, 10);
        advance_time(100);
        let (pre, _) = prepare(account_ext(OWNER, generation), OWNER, &normal()).unwrap();
        post(pre, &normal(), None);
        assert!(<Transaction as StorageInfoTrait>::storage_info().is_empty());
        assert!(!frame_support::storage::unhashed::contains_prefixed_key(
            &sp_io::hashing::twox_128(b"Transaction")
        ));
    });
}

// ---------------------------------------------------------------------------
// Node and NodeIdentity paths
// ---------------------------------------------------------------------------

#[test]
fn delegated_actor_charges_global_and_local_accounting_without_changing_the_origin() {
    new_test_ext().execute_with(|| {
        setup_node();
        set_local_rate(0, 10);
        let generation = subscribe(OWNER, 10);
        advance_time(100);

        let (pre, origin) = prepare(node_ext(0, generation), ACTOR, &normal()).unwrap();
        assert_eq!(global_available(OWNER), 1_000 - COST as u128);
        assert_eq!(local_available(OWNER, 0), Some(1_000 - COST as u128));
        // The call is dispatched as the actor, never as the owner.
        assert_eq!(frame_system::ensure_signed(origin).ok(), Some(ACTOR));

        post(pre, &normal(), None);
        assert_eq!(
            paid_events(),
            vec![pallet_transaction::Event::SubscriptionTransactionPaid {
                who: ACTOR,
                owner: OWNER,
                local_id: Some(ScopeId(0)),
                generation,
                usage: COST,
            }]
        );
    });
}

#[test]
fn unauthorized_actor_is_rejected() {
    new_test_ext().execute_with(|| {
        setup_node();
        set_local_rate(0, 10);
        let generation = subscribe(OWNER, 10);
        advance_time(100);
        assert_eq!(
            prepare(node_ext(0, generation), STRANGER, &normal()).err(),
            invalid(InvalidTransaction::BadSigner)
        );
        assert_eq!(
            prepare(node_ext(42, generation), ACTOR, &normal()).err(),
            invalid(InvalidTransaction::BadSigner)
        );
        assert_eq!(global_available(OWNER), 0);
    });
}

#[test]
fn missing_local_limit_rejects_node_usage_without_falling_back_to_global() {
    new_test_ext().execute_with(|| {
        setup_node();
        let generation = subscribe(OWNER, 10);
        advance_time(100);
        assert_eq!(
            prepare(node_ext(0, generation), ACTOR, &normal()).err(),
            invalid(InvalidTransaction::Payment)
        );
        // Nothing was charged: the stored bucket is untouched.
        assert_eq!(global_available(OWNER), 0);
    });
}

#[test]
fn local_rate_is_enforced_by_subscription() {
    new_test_ext().execute_with(|| {
        setup_node();
        set_local_rate(0, 4);
        let generation = subscribe(OWNER, 10);
        advance_time(100);
        // Local allowance 400 < 610 although the global one is 1_000.
        assert_eq!(
            prepare(node_ext(0, generation), ACTOR, &normal()).err(),
            invalid(InvalidTransaction::Payment)
        );
        advance_time(60);
        assert_ok!(prepare(node_ext(0, generation), ACTOR, &normal()));
    });
}

#[test]
fn local_overcommit_does_not_bypass_the_owner_global_rate() {
    new_test_ext().execute_with(|| {
        setup_node();
        set_local_rate(0, 1_000);
        let generation = subscribe(OWNER, 10);
        advance_time(100);
        assert_ok!(prepare(node_ext(0, generation), ACTOR, &normal()));
        assert_eq!(
            prepare(node_ext(0, generation), ACTOR, &normal()).err(),
            invalid(InvalidTransaction::Payment)
        );
    });
}

#[test]
fn node_identity_uses_the_canonical_node_context() {
    new_test_ext().execute_with(|| {
        setup_node();
        set_local_rate(0, 10);
        assert_ok!(Cps::set_identity(
            RuntimeOrigin::signed(OWNER),
            NodeId(0),
            IDENTITY,
            Some(pallet_robonomics_cps::NodeIdentityPermit {
                valid_until: 50,
                signature: TestSignature(IDENTITY, Cps::identity_permit_payload(ScopeId(0), 50)),
            }),
        ));
        let generation = subscribe(OWNER, 10);
        advance_time(1_000);

        let by_identity =
            Ext::subscription(AuthorizationContext::NodeIdentity(IDENTITY), generation);
        let (pre, _) = prepare(by_identity, ACTOR, &normal()).unwrap();
        post(pre, &normal(), None);
        assert_eq!(local_available(OWNER, 0), Some(10_000 - COST as u128));

        // The canonical Node context hits the very same local state.
        let (pre, _) = prepare(node_ext(0, generation), ACTOR, &normal()).unwrap();
        post(pre, &normal(), None);
        assert_eq!(local_available(OWNER, 0), Some(10_000 - 2 * COST as u128));
    });
}

#[test]
fn changing_node_identity_does_not_reset_local_accounting() {
    new_test_ext().execute_with(|| {
        setup_node();
        set_local_rate(0, 10);
        assert_ok!(Cps::set_identity(
            RuntimeOrigin::signed(OWNER),
            NodeId(0),
            OWNER,
            None
        ));
        let generation = subscribe(OWNER, 10);
        advance_time(100);
        assert_ok!(prepare(node_ext(0, generation), ACTOR, &normal()));
        let before = local_available(OWNER, 0);

        assert_ok!(Cps::clear_identity(RuntimeOrigin::signed(OWNER), NodeId(0)));
        assert_ok!(Cps::set_identity(
            RuntimeOrigin::signed(OWNER),
            NodeId(0),
            ACTOR,
            Some(pallet_robonomics_cps::NodeIdentityPermit {
                valid_until: 50,
                signature: TestSignature(ACTOR, Cps::identity_permit_payload(ScopeId(0), 50)),
            }),
        ));
        // The identity change leaves the local bucket as it was.
        assert_eq!(local_available(OWNER, 0), before);

        let by_identity =
            || Ext::subscription(AuthorizationContext::NodeIdentity(ACTOR), generation);
        // 390 left of 1_000 and the state was not reset to a full bucket.
        assert_eq!(
            prepare(by_identity(), OWNER, &normal()).err(),
            invalid(InvalidTransaction::Payment)
        );
        advance_time(100);
        assert_ok!(prepare(by_identity(), OWNER, &normal()));
        assert_eq!(local_available(OWNER, 0), Some(1_000 + 390 - COST as u128));
    });
}

// ---------------------------------------------------------------------------
// Correction
// ---------------------------------------------------------------------------

#[test]
fn post_dispatch_refunds_global_and_local_state_together() {
    new_test_ext().execute_with(|| {
        setup_node();
        set_local_rate(0, 10);
        let generation = subscribe(OWNER, 10);
        advance_time(100);
        let (pre, _) = prepare(node_ext(0, generation), ACTOR, &normal()).unwrap();
        // Actual weight 300 instead of 500: 200 is returned.
        post(pre, &normal(), Some(300));
        let actual = (BASE_EXTRINSIC + LEN as u64 + 300) as u128;
        assert_eq!(global_available(OWNER), 1_000 - actual);
        assert_eq!(local_available(OWNER, 0), Some(1_000 - actual));
    });
}

#[test]
fn correction_never_credits_a_newer_generation() {
    new_test_ext().execute_with(|| {
        let generation = subscribe(OWNER, 10);
        advance_time(100);
        let (pre, _) = prepare(account_ext(OWNER, generation), OWNER, &normal()).unwrap();

        // The dispatched call starts a new generation.
        let newer = subscribe(OWNER, 10);
        assert_eq!(newer, generation + 1);
        let before = global_available(OWNER);

        post(pre, &normal(), Some(0));
        assert_eq!(global_available(OWNER), before);
    });
}

// ---------------------------------------------------------------------------
// Fee mode
// ---------------------------------------------------------------------------

#[test]
fn fee_mode_withdraws_the_ordinary_fee_with_tip() {
    new_test_ext().execute_with(|| {
        pallet_transaction_payment::NextFeeMultiplier::<Runtime>::put(
            Multiplier::saturating_from_integer(2),
        );
        let info = normal();
        let tip = 7;
        let expected =
            pallet_transaction_payment::Pallet::<Runtime>::compute_fee(LEN as u32, &info, tip);
        assert!(expected > COST + tip);
        let before = Balances::free_balance(ACTOR);

        let (pre, origin) = prepare(Ext::fee(tip), ACTOR, &info).unwrap();
        assert_eq!(Balances::free_balance(ACTOR), before - expected);
        assert_eq!(frame_system::ensure_signed(origin).ok(), Some(ACTOR));
        post(pre, &info, Some(300));
        // The unused weight is refunded by the ordinary fee logic.
        assert!(Balances::free_balance(ACTOR) > before - expected);
        assert!(paid_events().is_empty());
    });
}

#[test]
fn fee_mode_does_not_touch_subscriptions() {
    new_test_ext().execute_with(|| {
        let _ = subscribe(OWNER, 10);
        advance_time(100);
        assert_ok!(prepare(Ext::fee(0), OWNER, &normal()));
        assert_eq!(global_available(OWNER), 0);
    });
}

#[test]
fn failed_subscription_payment_does_not_fall_back_to_fees() {
    new_test_ext().execute_with(|| {
        let generation = subscribe(OWNER, 10);
        let balance = Balances::free_balance(OWNER);
        // No allowance has accrued yet.
        assert_eq!(
            prepare(account_ext(OWNER, generation), OWNER, &normal()).err(),
            invalid(InvalidTransaction::Payment)
        );
        assert_eq!(Balances::free_balance(OWNER), balance);
    });
}

// ---------------------------------------------------------------------------
// Scheduling
// ---------------------------------------------------------------------------

#[test]
fn subscription_class_is_above_fee_class_whatever_the_tip() {
    new_test_ext().execute_with(|| {
        let generation = subscribe(OWNER, 10);
        advance_time(100);
        let subscription = validate(&account_ext(OWNER, generation), OWNER, &normal())
            .unwrap()
            .0
            .priority;
        for tip in [0, 1, 1_000, 900_000] {
            let fee = validate(&Ext::fee(tip), ACTOR, &normal())
                .unwrap()
                .0
                .priority;
            assert!(fee < subscription, "tip {tip}: {fee} >= {subscription}");
        }
        assert!(subscription >= priority::SUBSCRIPTION_BASE);
        assert!(subscription < priority::OPERATIONAL_BASE);
    });
}

#[test]
fn operational_class_is_above_subscription_class() {
    new_test_ext().execute_with(|| {
        let generation = subscribe(OWNER, 10);
        advance_time(100);
        let operational = info(WEIGHT, DispatchClass::Operational);
        let subscription = validate(&account_ext(OWNER, generation), OWNER, &normal())
            .unwrap()
            .0
            .priority;
        let operational_fee = validate(&Ext::fee(0), ACTOR, &operational)
            .unwrap()
            .0
            .priority;
        assert!(operational_fee >= priority::OPERATIONAL_BASE);
        assert!(operational_fee > subscription);
    });
}

#[test]
fn smaller_usage_ranks_higher_inside_the_subscription_class() {
    new_test_ext().execute_with(|| {
        let generation = subscribe(OWNER, 1_000);
        advance_time(100);
        let priority_of = |weight| {
            validate(
                &account_ext(OWNER, generation),
                OWNER,
                &info(weight, DispatchClass::Normal),
            )
            .unwrap()
            .0
            .priority
        };
        assert!(priority_of(100) > priority_of(200));
        assert_eq!(priority_of(100), priority_of(100));
    });
}

#[test]
fn priority_bands_are_disjoint() {
    assert_eq!(
        priority::fee_priority(DispatchClass::Normal, u64::MAX),
        priority::SUBSCRIPTION_BASE - 1
    );
    assert_eq!(
        priority::fee_priority(DispatchClass::Operational, u64::MAX),
        u64::MAX
    );
    assert_eq!(
        priority::subscription_priority(DispatchClass::Normal, 0),
        priority::OPERATIONAL_BASE - 1
    );
    assert_eq!(
        priority::subscription_priority(DispatchClass::Normal, u128::MAX),
        priority::SUBSCRIPTION_BASE
    );
}

// ---------------------------------------------------------------------------
// Encoding and weight
// ---------------------------------------------------------------------------

#[test]
fn payment_encoding_is_stable() {
    let fee = Ext::fee(7);
    assert_eq!(fee.encode(), vec![0, 7 << 2]);
    let subscription = account_ext(OWNER, 3);
    let mut expected = vec![1];
    expected.extend(AuthorizationContext::<u64, NodeId>::Account(OWNER).encode());
    expected.extend(3u64.encode());
    assert_eq!(subscription.encode(), expected);
    assert_eq!(Ext::default(), Ext::fee(0));
}

#[test]
fn subscription_payment_declares_the_extension_overhead() {
    new_test_ext().execute_with(|| {
        let subscription = account_ext(OWNER, 1).weight(&call());
        assert_eq!(
            subscription,
            <() as pallet_transaction::WeightInfo>::subscription_charge()
        );
        assert!(
            <Weights as Get<BlockWeights>>::get()
                .get(DispatchClass::Normal)
                .base_extrinsic
                .ref_time()
                > 0
        );
    });
}
