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

A Scope is the administrative and economic boundary of the CPS hierarchy.

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

Subscription generation GC
```

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

# 14. Architectural Principles

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
