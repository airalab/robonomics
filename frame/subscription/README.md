# pallet-robonomics-subscription

Runtime accounting layer for time-limited platform resources. The pallet
answers: which resources are allocated to this owner, for how long, and what
is the current accounting state? It has no extrinsics and is independent from
how a subscription is acquired, authorization, fee payment and resource
metering.

```text
allocator          -> SubscriptionAllocator -> pallet-robonomics-subscription
resource consumer  -> ResourceAccounting    -> pallet-robonomics-subscription
limit provider     -> ResourceLimit         -> pallet-robonomics-subscription
```

The shared interfaces live in
[`robonomics-subscription-primitives`](./primitives).

Allocation mechanisms live in separate pallets: [`governance`](./governance)
(privileged assignment) and [`purchase`](./purchase) (a fixed product
bought by a USDT deposit over XCM).

## Model

- **One generation per owner.** Every allocation replaces the current
  subscription and increments the owner's generation. The counter survives
  revocation and expiry. A charge made in generation `N` can never consume
  or refund generation `N + 1`.
- **Runtime time, not blocks.** `started_at = Time::now()`,
  `expires_at = started_at + valid_for`; a subscription is active while
  `now < expires_at`. Zero durations and timestamp overflow are rejected.
- **Transaction resource.** The allocation is a rate in `Wn` per millisecond
  (`Wn` is the smallest unit of XRT, `1 XRT = 10^9 Wn`), accounted with a
  token bucket:
  `available = min(available + rate * elapsed_ms, rate * TransactionBurstPeriod)`.
  A new generation starts with an empty bucket.
- **Global capacity.** `TotalAllocatedTransactionRate` is maintained in O(1)
  and never exceeds `MaxSubscriptionTransactionRate`. Replacement only
  accounts the rate delta.
- **Expiration.** A bounded `ExpirationAgenda` releases the global rate of
  expired subscriptions in `on_initialize`. Entries carry the generation, so
  stale entries never release a replacement's capacity. Consumption is
  rejected exactly from `expires_at`; capacity is released at most one
  `ExpirationBucketSize` later (more under load, bounded by
  `MaxExpirationsPerBlock`).
- **Local accounting.** State is keyed by `(owner, LocalId, resource)`,
  where `LocalId` is opaque. The local rate comes only from
  `Config::ResourceLimits`; a missing limit forbids local consumption. Local
  rates may overcommit the owner's rate, the global bucket is authoritative,
  and both buckets are charged atomically. Local state of an older
  generation is reset lazily on first use and accrues from the generation
  start.
- **Correction.** Consumption returns an opaque `Charge`. `correct` returns
  the difference between charged and actual usage to the same global (and
  local) bucket and fails for any other generation.
- **`Limit` is not `Usage`.** The `Limit` and `Usage` types are distinct
  enums so other resources (Storage, Compute) can add their own variants and
  accounting state.

The pallet does not use TPS or `free_weight`, and does not touch legacy
`pallet-robonomics-rws` storage.

## Weights

`weights::SubstrateWeight` contains conservative estimates derived from
storage access counts. Replace them with benchmarked weights. Resource
consumers are responsible for charging the weights of `consume_*` and
`correct`.

## Resource consumers

[`pallet-robonomics-transaction`](../transaction) consumes the Transaction
resource: it supplies the baseline `Usage` of each transaction and calls
`ResourceAccounting`. For advisory checks (transaction-pool validation) the
interface has read-only `can_consume_global` and `can_consume_local`, which give
the verdict `consume_*` would give against the current state and write
nothing.
