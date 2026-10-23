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
//! Unit tests: codec stability and trait flexibility, using mock types only.

use super::*;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::{traits::ConstU32, DispatchError, DispatchResult};

type AccountId = u64;

#[test]
fn resource_codec_indexes_are_stable() {
    assert_eq!(Resource::Transaction.encode(), vec![0]);
    assert_eq!(Resource::Storage.encode(), vec![1]);
    assert_eq!(Resource::Compute.encode(), vec![2]);
    for resource in [Resource::Transaction, Resource::Storage, Resource::Compute] {
        assert_eq!(Resource::decode(&mut &resource.encode()[..]), Ok(resource));
    }
    assert!(Resource::decode(&mut &[3u8][..]).is_err());
}

#[test]
fn authorization_context_codec_is_stable() {
    type Ctx = AuthorizationContext<AccountId, u32>;
    assert_eq!(Ctx::Account(7).encode()[0], 0);
    assert_eq!(Ctx::Node(7).encode()[0], 1);
    assert_eq!(Ctx::NodeIdentity(7).encode()[0], 2);
    for ctx in [Ctx::Account(7), Ctx::Node(8), Ctx::NodeIdentity(9)] {
        assert_eq!(Ctx::decode(&mut &ctx.encode()[..]), Ok(ctx));
    }
}

/// Authorizes against the authorization context itself and has no local
/// accounting.
struct DirectAuth;

impl ResourceAuthorization<AccountId> for DirectAuth {
    type ContextId = AccountId;
    type LocalId = ();
    type ResourceId = Resource;

    fn authorize(
        actor: &AccountId,
        context: &AccountId,
        _resource: Resource,
    ) -> Result<ResourceContext<AccountId, ()>, DispatchError> {
        if actor == context {
            Ok(ResourceContext {
                owner: *actor,
                local_id: None,
            })
        } else {
            Err(DispatchError::BadOrigin)
        }
    }
}

/// Maps nodes to scopes: the authorization context (`u32` node) differs from
/// the local accounting context (`u16` scope).
struct NodeAuth;

impl ResourceAuthorization<AccountId> for NodeAuth {
    type ContextId = AuthorizationContext<AccountId, u32>;
    type LocalId = u16;
    type ResourceId = Resource;

    fn authorize(
        actor: &AccountId,
        context: &Self::ContextId,
        _resource: Resource,
    ) -> Result<ResourceContext<AccountId, u16>, DispatchError> {
        match context {
            AuthorizationContext::Account(owner) if owner == actor => Ok(ResourceContext {
                owner: *owner,
                local_id: None,
            }),
            // Node 10 is in scope 1 owned by account 100; identity 50 is node 10.
            AuthorizationContext::Node(10) | AuthorizationContext::NodeIdentity(50) => {
                Ok(ResourceContext {
                    owner: 100,
                    local_id: Some(1),
                })
            }
            _ => Err(DispatchError::BadOrigin),
        }
    }
}

#[test]
fn authorization_supports_different_context_ids() {
    assert!(DirectAuth::authorize(&1, &1, Resource::Transaction).is_ok());
    assert!(DirectAuth::authorize(&1, &2, Resource::Transaction).is_err());

    let by_node = NodeAuth::authorize(&1, &AuthorizationContext::Node(10), Resource::Storage);
    let by_identity = NodeAuth::authorize(
        &1,
        &AuthorizationContext::NodeIdentity(50),
        Resource::Storage,
    );
    // Both address the same canonical node and share one accounting context.
    assert_eq!(by_node, by_identity);
}

#[test]
fn resource_context_distinguishes_global_and_local() {
    let global =
        NodeAuth::authorize(&5, &AuthorizationContext::Account(5), Resource::Compute).unwrap();
    assert_eq!(global.owner, 5);
    assert_eq!(global.local_id, None);

    let local =
        NodeAuth::authorize(&5, &AuthorizationContext::Node(10), Resource::Compute).unwrap();
    assert_eq!(local.owner, 100);
    assert_eq!(local.local_id, Some(1));
}

/// Transaction rate limit, in baseline XRT per second.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Encode, Decode)]
struct TransactionRate(u128);

/// Storage limit, in bytes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Encode, Decode)]
struct StorageLimit(u64);

/// Cost of one transaction, in baseline XRT.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct TransactionUsage(u128);

/// Runtime-defined heterogeneous limit.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Encode, Decode)]
enum RuntimeLimit {
    Transaction(TransactionRate),
    Storage(StorageLimit),
}

/// Runtime-defined heterogeneous usage; deliberately not the limit type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RuntimeUsage {
    Transaction(TransactionUsage),
    Storage(u64),
}

struct Limits;

impl ResourceLimit for Limits {
    type LocalId = u16;
    type ResourceId = Resource;
    type Limit = RuntimeLimit;

    fn limit(local: &u16, resource: Resource) -> Option<RuntimeLimit> {
        match (local, resource) {
            (1, Resource::Transaction) => Some(RuntimeLimit::Transaction(TransactionRate(3))),
            (1, Resource::Storage) => Some(RuntimeLimit::Storage(StorageLimit(1024))),
            _ => None,
        }
    }
}

#[test]
fn limit_type_differs_from_usage_type() {
    // The associated types are unrelated; this only compiles if `Limit` and
    // `Usage` are independent.
    fn limit_of<L: ResourceLimit>() -> core::any::TypeId
    where
        L::Limit: 'static,
    {
        core::any::TypeId::of::<L::Limit>()
    }
    assert_ne!(
        limit_of::<Limits>(),
        core::any::TypeId::of::<RuntimeUsage>()
    );

    assert_eq!(
        Limits::limit(&1, Resource::Transaction),
        Some(RuntimeLimit::Transaction(TransactionRate(3)))
    );
    assert_eq!(Limits::limit(&2, Resource::Transaction), None);
}

#[test]
fn allocation_is_bounded_and_carries_heterogeneous_limits() {
    type Allocation = SubscriptionAllocation<Resource, RuntimeLimit, u64, ConstU32<2>>;

    let transaction = ResourceAllocation {
        resource: Resource::Transaction,
        limit: RuntimeLimit::Transaction(TransactionRate(10)),
    };
    let storage = ResourceAllocation {
        resource: Resource::Storage,
        limit: RuntimeLimit::Storage(StorageLimit(1 << 30)),
    };

    let allocation = Allocation {
        valid_for: 3600,
        resources: vec![transaction.clone(), storage.clone()]
            .try_into()
            .unwrap(),
    };
    assert_eq!(allocation.resources.len(), 2);
    assert_eq!(
        Allocation::decode(&mut &allocation.encode()[..]),
        Ok(allocation)
    );

    let too_many = vec![transaction.clone(), storage.clone(), transaction];
    assert!(sp_runtime::BoundedVec::<_, ConstU32<2>>::try_from(too_many.clone()).is_err());

    // Oversized encodings are rejected on decode as well.
    let encoded = (3600u64, too_many).encode();
    assert!(Allocation::decode(&mut &encoded[..]).is_err());
}

#[test]
fn allocation_max_encoded_len_is_bounded() {
    use parity_scale_codec::MaxEncodedLen;
    type Allocation = SubscriptionAllocation<Resource, u128, u64, ConstU32<4>>;
    // Resource (1) + u128 (16) per entry; 4 entries; compact length prefix; u64.
    let entry = 1 + 16;
    assert!(Allocation::max_encoded_len() >= 8 + 4 * entry);
    assert!(Allocation::max_encoded_len() < 8 + 4 * entry + 8);
}

/// Mock accounting with an explicit current generation and a ledger of
/// charged usage per `(generation, source)`.
struct Ledger;

type Charge = ResourceCharge<AccountId, u16, Resource, ResourceGeneration, RuntimeUsage>;

std::thread_local! {
    static GENERATION: core::cell::Cell<ResourceGeneration> = const { core::cell::Cell::new(1) };
    static CREDITED: core::cell::RefCell<Vec<(ResourceGeneration, ChargeSource<u16>, RuntimeUsage)>> =
        const { core::cell::RefCell::new(Vec::new()) };
}

impl ResourceAccounting<AccountId> for Ledger {
    type LocalId = u16;
    type ResourceId = Resource;
    type Usage = RuntimeUsage;
    type Generation = ResourceGeneration;
    type Charge = Charge;

    fn generation(owner: &AccountId) -> Option<ResourceGeneration> {
        (*owner != 0).then(|| GENERATION.with(|g| g.get()))
    }

    fn can_consume_global(
        owner: &AccountId,
        generation: ResourceGeneration,
        resource: Resource,
        usage: RuntimeUsage,
    ) -> DispatchResult {
        Self::charge(owner, ChargeSource::Global, generation, resource, usage).map(|_| ())
    }

    fn can_consume_local(
        owner: &AccountId,
        local: &u16,
        generation: ResourceGeneration,
        resource: Resource,
        usage: RuntimeUsage,
    ) -> DispatchResult {
        Self::charge(
            owner,
            ChargeSource::Local(*local),
            generation,
            resource,
            usage,
        )
        .map(|_| ())
    }

    fn consume_global(
        owner: &AccountId,
        generation: ResourceGeneration,
        resource: Resource,
        usage: RuntimeUsage,
    ) -> Result<Charge, DispatchError> {
        Self::charge(owner, ChargeSource::Global, generation, resource, usage)
    }

    fn consume_local(
        owner: &AccountId,
        local: &u16,
        generation: ResourceGeneration,
        resource: Resource,
        usage: RuntimeUsage,
    ) -> Result<Charge, DispatchError> {
        Self::charge(
            owner,
            ChargeSource::Local(*local),
            generation,
            resource,
            usage,
        )
    }

    fn correct(charge: Charge, actual_usage: RuntimeUsage) -> DispatchResult {
        // A stale receipt must be rejected rather than credit the new epoch.
        if charge.generation != GENERATION.with(|g| g.get()) {
            return Err(DispatchError::Other("stale generation"));
        }
        CREDITED.with(|c| {
            c.borrow_mut()
                .push((charge.generation, charge.source, actual_usage))
        });
        Ok(())
    }
}

impl Ledger {
    fn charge(
        owner: &AccountId,
        source: ChargeSource<u16>,
        generation: ResourceGeneration,
        resource: Resource,
        usage: RuntimeUsage,
    ) -> Result<Charge, DispatchError> {
        if Self::generation(owner) != Some(generation) {
            return Err(DispatchError::Other("wrong generation"));
        }
        Ok(ResourceCharge {
            owner: *owner,
            generation,
            resource,
            source,
            charged: usage,
        })
    }
}

#[test]
fn accounting_consumes_runtime_usage_and_preserves_context() {
    let usage = RuntimeUsage::Transaction(TransactionUsage(2));
    let generation = Ledger::generation(&1).unwrap();

    let global = Ledger::consume_global(&1, generation, Resource::Transaction, usage).unwrap();
    assert_eq!(global.generation, generation);
    assert_eq!(global.source, ChargeSource::Global);
    assert_eq!(global.charged, usage);

    let local = Ledger::consume_local(
        &1,
        &1,
        generation,
        Resource::Storage,
        RuntimeUsage::Storage(64),
    )
    .unwrap();
    assert_eq!(local.source, ChargeSource::Local(1));
    assert_eq!(local.resource, Resource::Storage);
    assert_eq!(local.owner, 1);

    assert_eq!(Ledger::generation(&0), None);
}

#[test]
fn admission_queries_mirror_consumption() {
    GENERATION.with(|g| g.set(3));
    let usage = RuntimeUsage::Transaction(TransactionUsage(2));

    assert_eq!(
        Ledger::can_consume_global(&1, 3, Resource::Transaction, usage),
        Ok(())
    );
    assert_eq!(
        Ledger::can_consume_local(&1, &7, 3, Resource::Transaction, usage),
        Ok(())
    );
    // A stale generation is rejected by the query exactly like by the charge.
    assert!(Ledger::can_consume_global(&1, 2, Resource::Transaction, usage).is_err());
    assert!(Ledger::consume_global(&1, 2, Resource::Transaction, usage).is_err());
    assert!(Ledger::can_consume_local(&0, &7, 3, Resource::Transaction, usage).is_err());
}

#[test]
fn correction_is_bound_to_original_generation() {
    GENERATION.with(|g| g.set(5));
    CREDITED.with(|c| c.borrow_mut().clear());

    let charge = Ledger::consume_global(
        &1,
        5,
        Resource::Transaction,
        RuntimeUsage::Transaction(TransactionUsage(2)),
    )
    .unwrap();

    // Same generation: the correction is applied to generation 5.
    let actual = RuntimeUsage::Transaction(TransactionUsage(1));
    assert_eq!(Ledger::correct(charge.clone(), actual), Ok(()));
    assert_eq!(
        CREDITED.with(|c| c.borrow().clone()),
        vec![(5, ChargeSource::Global, actual)]
    );

    // A new generation starts: the old receipt must not credit it.
    GENERATION.with(|g| g.set(6));
    assert!(Ledger::correct(charge.clone(), actual).is_err());
    assert_eq!(CREDITED.with(|c| c.borrow().len()), 1);

    // Consumption must name the current generation.
    assert!(Ledger::consume_global(&1, 5, Resource::Transaction, actual).is_err());
}

#[test]
fn charge_receipt_codec_round_trips() {
    let charge: ResourceCharge<AccountId, u16, Resource, ResourceGeneration, u128> =
        ResourceCharge {
            owner: 1,
            generation: 9,
            resource: Resource::Compute,
            source: ChargeSource::Local(4),
            charged: 42,
        };
    assert_eq!(Decode::decode(&mut &charge.encode()[..]), Ok(charge));
}

/// Allocator that only validates the shape of what it is handed.
struct Allocator;

impl SubscriptionAllocator<AccountId> for Allocator {
    type ResourceId = Resource;
    type Limit = RuntimeLimit;
    type Moment = u64;
    type MaxResources = ConstU32<3>;

    fn allocate(
        _owner: &AccountId,
        allocation: SubscriptionAllocation<Resource, RuntimeLimit, u64, ConstU32<3>>,
    ) -> DispatchResult {
        if allocation.resources.is_empty() {
            return Err(DispatchError::Other("empty allocation"));
        }
        Ok(())
    }

    fn revoke(_owner: &AccountId) -> DispatchResult {
        Ok(())
    }
}

#[test]
fn allocator_accepts_normalized_allocation() {
    let allocation = SubscriptionAllocation {
        valid_for: 60,
        resources: vec![ResourceAllocation {
            resource: Resource::Transaction,
            limit: RuntimeLimit::Transaction(TransactionRate(1)),
        }]
        .try_into()
        .unwrap(),
    };
    assert_eq!(Allocator::allocate(&1, allocation), Ok(()));
    assert_eq!(Allocator::revoke(&1), Ok(()));
}
