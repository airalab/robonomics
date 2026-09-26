# Robonomics 5.0 Roadmap

Robonomics 5.x evolves the network into infrastructure protocol for cyber-physical systems.

The architecture is built around these core layers:

```text
Governance
    global protocol rules

CPS
    topology
    canonical state
    Scope
    Access

Subscription
    resource entitlement
    accounting

Storage
    persistent content-addressed data

Compute
    verifiable execution

Policy
    verified state transitions
```

The execution model is:

```text
Storage records facts.

Compute derives and proves facts.

Policy decides when facts may change CPS state.

Runtime commits the transition.
```

The product principle behind the resource layers is:

> **Robonomics is an IoT cloud, not a fee market. A Subscription buys predictable transaction latency, and subscribers within network capacity never compete with each other for block space.**

An alarm or a door lock must act within a known time window even when the network is busy. In a fee market, whoever pays more under congestion wins. Robonomics instead admits only as many Subscriptions as the network can serve and protects their share of every block.

---

# 1. Runtime-first Architecture

The Robonomics core focuses on:

```text
runtime
FRAME pallets
protocol state transitions
CPS
resource accounting
Storage
Compute
Policy
runtime upgrades
```

Operational tooling, workers and developer utilities remain outside the protocol core.

`polkadot-omni-node` is the default node foundation wherever custom node-side functionality is not required.

---

# 2. Community Governance

Governance controls protocol evolution and global network parameters.

```text
Governance
    |
    +-- runtime upgrades
    +-- Subscription economics
    +-- Transaction parameters
    +-- Storage parameters
    +-- Compute parameters
    +-- retention policy
    +-- proof / verification policy
    `-- emergency controls
```

In particular, governance defines how Subscription units translate into protocol resources:

```text
SubscriptionUnit
    |
    +-- TransactionBudgetRate
    +-- Storage entitlement
    `-- Compute entitlement
```

Governance changes global protocol parameters.

It does not manage individual:

```text
Scopes
Access grants
Subscriptions
Storage objects
Compute jobs
```

Those remain ordinary runtime state governed by their respective protocol rules.

---

# 3. CPS

CPS describes the topology and canonical state of cyber-physical infrastructure.

A node has a stable structural identity:

```text
NodeId
    |
    +-- Parent
    +-- Meta
    `-- Payload
```

`Parent` is immutable.

Relocation is represented by creating a new node rather than moving an existing `NodeId`.

The hierarchy is bounded so that every walk up the tree has a known worst-case cost:

```text
MaxDepth      30 levels
MaxChildren   100 children per node
```

Exact values are runtime constants and may be tuned after benchmarking.

## Meta

`meta` contains durable description and configuration:

```text
type
schema
model
firmware
calibration
configuration
content references
```

## Payload

`payload` contains compact current state:

```text
status
mode
latest value
latest content reference
current operational state
```

Large and historical data belongs in Storage, not CPS state.

---

# 4. Scope

A Scope is the administrative and economic boundary of the CPS hierarchy: an owner, a Subscription payer and local resource limits.

The closest cloud analogy:

```text
Scope             ~ project
ScopeResources    ~ project quotas
Access            ~ API keys
Subscription      ~ billing account
```

Unlike most clouds, limits live only on a Scope, not on individual Access grants.

Different subtrees may have different owners. Example:

```text
building                 Scope A, owner: building operator
  +-- floor 1
  |     +-- apartment 3  Scope B, owner: resident
  |     |     +-- sensor
  |     |     `-- sensor
  |     `-- corridor sensor
  `-- roof station
```

The operator has no authority inside Scope B. An installer can work in Scope A through Access without owning it.

```text
NodeId
   |
   v
ActiveScope
   |
   v
ScopeId
   |
   +-- root
   +-- owner
   +-- resources
   `-- Access
```

Conceptually:

```text
ActiveScope
    NodeId -> ScopeId

ScopeRoot
    ScopeId -> NodeId

ScopeOwner
    ScopeId -> AccountId

ScopeResources
    ScopeId x Resource -> Limit
```

A node resolves to the nearest active Scope ancestor:

```text
resolve_scope(node)
=
nearest ancestor with ActiveScope
```

`resolve_scope` is exposed as a runtime API returning `ScopeId`, from which clients read the root, owner and limits.

Scope resolution runs during transaction validation, before any fee is charged. In the worst case it reads one storage item per level, up to `MaxDepth`. Checking authority must never cost a comparable amount to the useful action itself, so the worst case must be benchmarked on both `ref_time` and `proof_size`, and caching of the resolved Scope considered if it is too expensive.

Nested Scopes are hard boundaries for:

```text
authority
Access
resource limits
accounting context
Subscription payer
```

Creating a new Scope on an existing Scope root replaces the active generation.

Deleting a non-root Scope removes that boundary and makes its subtree inherit the nearest parent Scope.

`ScopeId` is globally unique and never reused.

Open: who may delete a nested Scope, and whether an issued `CreateScope` grant can be revoked before it is used.

---

# 5. Access and Capabilities

Access delegates authority inside one Scope:

```text
Access
    ScopeId x (
        NodeId,
        AccountId,
        Capability
    )
    -> inherited
```

```text
inherited = false
    exact node

inherited = true
    node + descendants
    inside the same Scope
```

Access never crosses a nested Scope boundary.

The Scope owner has implicit authority inside the Scope.

Capability families include:

```text
State authority

    Write
        mutate CPS meta / payload


Resource authority

    Transaction
        consume Scope transaction resource

    Storage
        consume Scope storage resource

    Compute
        consume Scope compute resource


Administration

    CreateScope
        create a replacement Scope
        on the exact Scope root
```

Capabilities remain independent.

For example:

```text
Write + Transaction

Storage + Transaction

Compute + Transaction

Storage + Write + Transaction
```

do not require combined capability variants.

Changing Scope control is implemented by granting `CreateScope` to the future owner, who creates a fresh Scope generation.

There is no `Read` capability. CPS state on chain is public. Confidentiality of device data is a matter of encryption or an off-chain layer, not of Access.

## Example: owner-funded sensors

This flow replaces today's RWS `set_devices`, at the level of individual nodes rather than a flat device list:

```text
owner wins a Subscription at auction
        |
        v
owner creates the CPS tree
        |
        v
owner grants Write + Transaction on sensor nodes
        |
        v
sensor signs a transaction for its NodeId
with payment mode Subscription(Scope)
        |
        v
runtime checks Scope, owner, Access and remaining resource
at transaction-pool entry and again before block inclusion
```

The sensor holds no XRT and needs no Subscription of its own.

Open: whether the transaction is signed by the sensor itself or by an edge gateway on its behalf. This decides which account receives Access.

---

# 6. Subscription

Subscription is the global resource layer associated with an account.

```text
AccountId
    |
    v
ActiveSubscription
    |
    v
SubscriptionId
```

`AccountId` is the stable subscription identity.

`SubscriptionId` is an internal generation and accounting identity.

Only one Subscription generation may be active for an account at a time.

A Subscription has one source:

```text
RWS lock
OR
Auction
```

Both sources produce normalized resource entitlement.

Changing entitlement creates a new `SubscriptionId` rather than mutating the old accounting generation.

```text
Subscription #42
        |
        | update
        v
Subscription #57

ActiveSubscription[owner] = #57
```

The old generation immediately becomes inactive and may be garbage-collected later.

Subscription answers:

> How much resource does this account have and how much may still be consumed?

It does not define authorization.

## Budget

An account has at most one active Subscription. `Subscription(Account)` and `Subscription(Scope)` are two ways of spending it, not two products.

For an Auction source the budget is virtual and XRT-denominated:

```text
budget per period
=
burned winning bid x SubscriptionBudgetMultiplier
```

- the multiplier is a governance parameter, on the order of 10-100x;
- the budget expires at the end of the period and does not accumulate;
- it is shared by Transaction, Storage and Compute accounting, but the transaction rate is capped independently of its size (see section 7).

The budget is expressed in XRT, not in weight, because the real cost of a parachain transaction depends on weight, proof size and block fullness.

The target retail price of a Subscription is about $1. Auction prices float in XRT, so this target has to be met through the multiplier and auction parameters, not assumed.

Open: what happens to the surplus of a winner who bid more than others. Options are equal allocation per Subscription with the surplus usable only for Storage and Compute, or a budget proportional to the bid.

---

# 7. Transaction Resource

Transaction resource is priced in XRT.

Subscription does not introduce an alternative transaction price.

The same runtime pricing is used for all payment modes:

```text
transaction
    |
    v
standard transaction-payment pricing
    |
    +--> real XRT
    |
    `--> virtual XRT Subscription budget
```

The standard pricing model remains responsible for:

```text
base fee
encoded length
weight
fee multiplier
post-dispatch correction
```

A user explicitly chooses the payment mode:

```text
Token

Subscription(Account)

Subscription(Scope)
```

## Token

The transaction is paid from the signer's real XRT balance.

## Subscription(Account)

The transaction is paid from the signer's active Subscription.

## Subscription(Scope)

The transaction is paid from the active Subscription of the resolved Scope owner.

Conceptually:

```text
RuntimeCall
    |
    v
CPS NodeId
    |
    v
Scope
    |
    v
ScopeOwner
    |
    v
ActiveSubscription
    |
    v
TransactionCredit
```

Scope-funded transaction payment requires `Transaction` Access.

Transaction accounting uses pre-charge and post-dispatch correction:

```text
validate
    read-only check

prepare
    reserve predicted XRT cost

dispatch

post_dispatch
    refund predicted - actual
```

Subscription transaction credit is virtual XRT-denominated capacity. It is not transferable or withdrawable XRT.

The Token mode stays as the default Substrate path and the fallback for accounts without a Subscription. Because the Subscription budget is a multiple of the burned bid, paying with real XRT is several times more expensive for the same transaction. Reference point: without a Subscription, 1 XRT should buy on the order of 10-20 datalog transactions.

## Latency guarantees

Predictable latency is what a Subscription sells. The following rules are required for it to hold:

```text
TransactionBudgetRate
    a hard per-Subscription rate cap,
    independent of remaining budget

Sum of all active rates
    bounded by a governance-set share of block capacity;
    no new Subscriptions beyond it

Block space reservation
    Subscription traffic has a reserved share or priority
    over Token-mode traffic

Fee multiplier
    frozen for Subscription-paid transactions,
    otherwise budgets lose value exactly under congestion

Two-dimensional weight
    capacity is reserved in both ref_time and proof_size,
    while users see a single unit
```

The guarantee is expressed as a latency corridor, for example inclusion within 2 / 4 / 6 / 10 seconds, to be measured and published per release.

---

# 8. Storage

Storage provides persistent content-addressed data without placing bulk bytes into ordinary FRAME state.

The core transition is CPS-aware:

```text
Storage::store(
    node,
    data
)
```

The CPS node determines:

```text
Scope
authorization
resource payer
semantic context
```

Storage consumes the Storage resource of the resolved Scope.

Transaction execution may independently be paid through:

```text
Token
Subscription(Account)
Subscription(Scope)
```

Bulk bytes are persisted through transaction storage and retrieved through content-addressed network protocols.

Conceptually:

```text
Runtime state
    commitments
    retention metadata
    accounting

Transaction storage
    bulk bytes

Network
    retrieval
```

Storage does not automatically modify CPS `payload`.

A state transition that records a resulting CID in `payload` additionally requires `Write` authority or an applicable Policy transition.

Large objects are chunked by client tooling and represented by manifests.

Storage lifecycle:

```text
store
  ->
persist
  ->
retrieve
  ->
renew
  ->
expire
```

---

# 9. Compute

Compute provides chain-coordinated verifiable execution.

Heavy computation runs outside the runtime.

```text
Compute::request(node, ...)
        |
        v
ComputeJob
        |
        v
worker / prover
        |
        v
RISC Zero
        |
        v
Receipt
        |
        v
Compute::submit(...)
```

Every job belongs to a CPS context.

The resolved Scope determines:

```text
authorization
resource payer
Subscription context
```

Compute requests consume Compute resource independently from Transaction resource.

Jobs should commit to the CPS state they operate on:

```text
NodeId
Meta hash
Payload hash
input commitments
```

A Compute result is a verified fact.

It does not automatically imply an arbitrary CPS state mutation.

---

# 10. Policy

Policy connects verified facts to canonical CPS transitions.

```text
Storage facts
      |
      v
Compute
      |
      v
verified result
      |
      v
Policy
      |
      v
CPS state transition
```

Policies may constrain transitions using:

```text
verified Compute result
expected previous CPS state
configured program / ImageID
```

State-changing transitions must be protected against stale proofs and concurrent state changes.

The initial Policy model should remain intentionally small.

---

# 11. Resource Model

The resource model separates authority, local Scope policy and global Subscription accounting.

```text
Access
    WHO may consume

ScopeResources
    local Scope limit

Subscription
    global owner entitlement
    mutable accounting
```

For a resource-consuming operation:

```text
NodeId
   |
   v
ScopeId
   |
   +--> Access
   |
   +--> ScopeResources
   |
   v
ScopeOwner
   |
   v
ActiveSubscription
   |
   v
Subscription accounting
```

Transaction, Storage and Compute maintain independent accounting even when they use XRT as a common economic denomination.

One account may own multiple Scopes, all consuming the same active Subscription.

---

# 12. Generations and Garbage Collection

Scope and Subscription use the same generation pattern:

```text
stable identity
    |
    v
active generation pointer
    |
    v
globally unique generation ID
```

```text
NodeId
    -> ActiveScope
    -> ScopeId

AccountId
    -> ActiveSubscription
    -> SubscriptionId
```

Replacement is logically immediate and does not require subtree/state rewrites.

Old generations become stale and are cleaned asynchronously.

Background GC should:

```text
enqueue stale generation
        |
        v
bounded on_idle cleanup
```

Authorization and accounting correctness must never depend on GC completion.

---

# 13. Implementation Sequence

## Phase 1 - Runtime, Governance and CPS Foundation

```text
runtime / governance foundation

CPS node model
immutable hierarchy

Scope
Access
Write capability

Scope generation GC
```

## Phase 2 - Subscription and Transactions

```text
account-scoped Subscription generations

RWS-backed Subscription
Auction-backed Subscription

XRT-denominated Transaction resource

unified transaction payment extension

Scope-funded Transaction capability

latency guarantees: rate caps, capacity bound, block space reservation

migration of RWS subscriptions and set_devices

Subscription generation GC
```

The first release of the new model covers the Transaction resource only. Storage follows, Compute is deferred.

Every runtime release must pass an end-to-end regression test on a fresh Subscription:

```text
new Subscription -> grant device access -> first device transaction
```

Devices activated on older Subscriptions can hide a broken activation path, so this test must never reuse existing Subscriptions.

## Embedded clients

Devices should sign and submit full transactions themselves, including the payment-mode extension. Firmware cannot parse runtime metadata, so the runtime repository publishes a normalized static description of the runtime and code generation happens at build time:

```text
robonomics-runtime-metadata     canonical metadata (#663)
    `-- robonomics-runtime-embed-api
            static runtime model (#662)
                |
                v
        embed-codegen in robins
            C bindings for selected calls (#666)
                |
                v
        API surface fingerprints
            firmware compatibility across upgrades (#667)
```

Firmware only needs an update when the part of the runtime it actually uses changes, not on every `spec_version` bump.

## Phase 3 - Storage

```text
Bulletin transaction-storage integration

Storage::store(node, data)

Storage capability and accounting

content addressing

retention / renewal

chunking and manifests
```

## Phase 4 - Compute

```text
Compute resource accounting

ComputeJob

RISC Zero proving

state snapshots

receipt verification

Compute results
```

## Phase 5 - Programmable CPS

```text
Policy model

proof-gated transitions

state-version protection

automatic CPS transitions
```

## Phase 6 - Applications

```text
Storage + Compute pipelines

sensors.social

robotics

industrial / scientific systems

additional protocol capabilities
```

---

# 14. Open Questions

```text
auction surplus
    equal allocation, or budget proportional to the bid

SubscriptionBudgetMultiplier
    value and who sets it (proposed: governance parameter)

per-delegate limits
    "this key: 2 transactions per day, that one: 5",
    with accounting per key; today limits exist only per Scope

latency under load
    fee multiplier, two-dimensional weight and uncapped Token traffic

nested Scope lifecycle
    who may delete it; can CreateScope be revoked

independent payers in a shared tree
    require nested Scopes, which removes the network
    administrator's authority inside them

device vs gateway signing
    decides which account receives Access

RWS migration
    moving existing subscriptions and set_devices to the new model
```

---

# 15. Architectural Principles

```text
Governance
    global protocol rules

NodeId
    structural CPS identity

ScopeId
    administrative / economic generation

SubscriptionId
    resource accounting generation

Write
    authority to modify CPS state

Transaction
    authority to consume Scope transaction resource

Storage
    authority to consume Scope storage resource

Compute
    authority to consume Scope compute resource

Subscription
    global resource entitlement of an account
```

The central separation is:

```text
Governance
    defines global protocol rules

CPS
    defines structure and canonical state

Scope
    defines administrative and economic boundary

Access
    defines local authority

Subscription
    defines available resources

Storage
    records persistent facts

Compute
    proves derived facts

Policy
    turns verified facts into CPS state transitions
```

> **Robonomics coordinates authority, resources, persistent data and verifiable computation around stable CPS contexts.**
