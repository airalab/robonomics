# robonomics-subscription-primitives

Shared types and traits for resource subscriptions. The crate defines
interfaces only: no storage, no pallet logic, and no dependency on CPS,
`pallet-robonomics-subscription`, `pallet-robonomics-transaction`, or any
allocator.

```text
robonomics-subscription-primitives
    -> authorization providers   (ResourceAuthorization, ResourceLimit)
    -> pallet-subscription       (ResourceAccounting, SubscriptionAllocator)
    -> resource consumers        (consume authorization and accounting)
    -> subscription allocators   (consume SubscriptionAllocator)
```

## Contents

| Item | Purpose |
|------|---------|
| `Resource`, `ResourceGeneration` | Stable resource identifiers (`Transaction = 0`, `Storage = 1`, `Compute = 2`) and the accounting epoch type |
| `AuthorizationContext` | `Account`, `Node`, or `NodeIdentity` context in which an actor uses a resource |
| `ResourceAuthorization` | Maps `(actor, ContextId, resource)` to `ResourceContext { owner, local_id }` |
| `ResourceLimit` | Read-only lookup of the `Limit` for `(LocalId, resource)` |
| `SubscriptionAllocation`, `SubscriptionAllocator` | Bounded, storage-independent allocation payload and the interface allocators use to create or revoke a subscription |
| `ResourceAccounting` | Global and local consumption with generation-bound correction via an opaque `Charge`, plus read-only `can_consume_*` admission queries |
| `ResourceCharge`, `ChargeSource` | Reference shape of a charge receipt |

## Invariants

- Resource SCALE indexes are stable and never reused.
- `Limit` (entitlement) and `Usage` (cost of one operation) are distinct
  types; nothing assumes `Limit == Usage` or a rate-based resource.
- `ContextId` (authorization context, for example a CPS `NodeId`) and
  `LocalId` (local accounting context, for example a CPS `ScopeId`) are
  distinct. Node identity is an authorization address only and never
  becomes a `LocalId`.
- `local_id = None` means global accounting only; `Some(..)` means global
  and local accounting are both required.
- Consumers supply `Usage` only; the local limit comes from `ResourceLimit`.
- A correction from generation `N` must never credit generation `N + 1`.
- `can_consume_*` never mutates state and mirrors the verdict of the
  matching `consume_*`; only `consume_*` is authoritative.
