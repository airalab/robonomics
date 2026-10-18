# pallet-robonomics-subscription-governance

Privileged allocation mechanism for
[`pallet-robonomics-subscription`](../README.md): a configurable origin assigns
an arbitrary normalized `SubscriptionAllocation` to a beneficiary. Use it for
bootstrap allocations, grants, testing and emergency provisioning.

```text
Config::AllocateOrigin -> allocate(beneficiary, allocation)
                       -> SubscriptionAllocator::allocate(...)
```

## Extrinsics

| Call | Description |
| --- | --- |
| `allocate(beneficiary, allocation)` | Forward the allocation to the allocator. Replaces the beneficiary's current generation. |
| `revoke(beneficiary)` | Revoke the beneficiary's current subscription. |

## Design

- The pallet has no storage and only decides *who*, *for how long* and *with
  which limits*. Generations, timestamps, capacity admission and accounting
  state belong to the subscription pallet.
- The allocator is a `Config` type implementing `SubscriptionAllocator`, so
  the pallet knows nothing about subscription storage, CPS, or resource
  consumers, and makes no assumption that a resource is rate-based.
- Allocation errors (invalid allocation, `GlobalRateCapacityExceeded`, ...)
  are returned unchanged and leave no state behind.
- The origin is runtime policy (`Root`, a council, ...); nothing is
  hard-coded.

## Weights

`weights::SubstrateWeight` contains conservative estimates that include the
allocator's work. Replace them with benchmarked weights.
