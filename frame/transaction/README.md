# pallet-robonomics-transaction

The first consumer of Subscription resources. It routes the payment of a
transaction between the ordinary fee market and the `Transaction` resource
of a subscription.

The pallet has no extrinsics and **no storage**. Its entry point is the
`ChargeTransaction` transaction extension, which replaces
`pallet_transaction_payment::ChargeTransactionPayment` in the runtime
`TxExtension`.

```text
ChargeTransaction
    Fee { tip }                            -> pallet-transaction-payment (unchanged)
    Subscription { context, generation }   -> ResourceAuthorization
                                              -> ResourceAccounting (pallet-robonomics-subscription)
```

It owns the baseline `Usage` of a transaction, payment routing, authorization
and generation checks, pre-dispatch charging, post-dispatch correction, and
pool priority classes. It does not own the subscription lifecycle, allocations,
token-bucket state, local limits, CPS topology, or allocators.

## Payment mode

The mode is part of the signed transaction; there is no silent fallback
between the two modes.

| Mode | SCALE | Meaning |
|------|-------|---------|
| `Fee { tip }` | index `0`, compact `tip` | baseline cost, times the congestion multiplier, plus `tip`, withdrawn from the signer |
| `Subscription { context, generation }` | index `1` | charged to the `Transaction` resource of a subscription |

`context` is an `AuthorizationContext`:

| Context | Authorization result | Accounting |
|---------|----------------------|------------|
| `Account(a)` | the signer is `a`; `owner = a`, no local context | global only |
| `Node(n)` | the signer may use the Transaction resource at node `n` (CPS `Capability::Transaction`) | global and local (`ScopeId`) |
| `NodeIdentity(a)` | `a` is resolved to its canonical node, then as `Node` | global and local (`ScopeId`) |

The node is the authorization context; the Scope is the local accounting
context. A node identity never becomes the accounting identifier, so changing
it does not reset local accounting. The pallet never calls CPS scope
resolution itself.

## Usage

The Subscription usage is the baseline cost

```text
base fee + length fee + weight fee
```

computed with the fee primitives of `pallet-transaction-payment` but **without**
the congestion multiplier and the tip. The same call, length and weight cost
the same usage under any network load. Transactions that do not pay fees cost
nothing. The Transaction `Limit` is a rate that the subscription pallet applies
(`pallet-robonomics-subscription`); this pallet only supplies the usage.

## Pipeline

1. **validate** (read-only, advisory): authorize the signer, check the signed
   generation against the current one, ask `can_consume_*` whether the
   estimated usage currently fits, and assign the pool priority. Several
   pending transactions may pass while competing for one allowance.
2. **prepare** (authoritative): authorize and check the generation again, then
   charge `consume_global` (no local context) or `consume_local`. The opaque
   charge is kept for correction.
3. **dispatch**: the origin is unchanged. A delegated actor dispatches the call
   as itself, never as the subscription owner.
4. **post-dispatch**: `correct` replaces the estimate by the actual usage. The
   charge binds the correction to the original generation, so a call that
   starts a newer generation never has it credited. A failed correction keeps
   the estimate charged and does not invalidate the executed transaction.

A rate-limited transaction fails with `InvalidTransaction::Payment`: the
current allowance is insufficient, not permanently exhausted, and the same
transaction may become admissible once the bucket refills. Other rejections use
`BadSigner` (authorization failed) and `Custom(NO_ACTIVE_SUBSCRIPTION)` or
`Custom(GENERATION_MISMATCH)`.

## Scheduling

`TransactionPriority` is split into disjoint bands (see `priority`):

```text
Operational   [2^63, u64::MAX]
Subscription  [2^62, 2^63)     smaller baseline usage ranks higher; no tip involved
Fee           [0,    2^62)     the ordinary priority, clamped to its band
```

A Fee tip can therefore never cross the Subscription boundary. The bands only
order the pool: block weight, length and the Operational reserve stay enforced
by `CheckWeight`, so Fee transactions use all capacity that Subscription
transactions leave free and no space is reserved when there is no Subscription
demand. The bands assume that the other runtime extensions add no priority.

The guarantee is a canonical block-production policy, not protection against a
malicious block author. The global rate cap of `pallet-robonomics-subscription`
bounds the sustained Subscription throughput.

## Configuration

| Item | Purpose |
|------|---------|
| `ContextId` | signed authorization context |
| `LocalId` | local accounting context |
| `Authorization` | `ResourceAuthorization` provider (CPS in the Robonomics runtime) |
| `Accounting` | `ResourceAccounting` (`pallet-robonomics-subscription`) |
| `ToUsage` | baseline cost to the accounting usage |
| `WeightInfo` | overhead of the extension |

Introducing the extension changes the transaction format: bump the runtime
`transaction_version`, regenerate the metadata and the client types.

## Weights

`weights::SubstrateWeight` contains conservative estimates derived from storage
access counts. Replace them with benchmarked weights.
