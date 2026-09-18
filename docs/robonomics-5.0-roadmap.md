# Robonomics 5.0 Roadmap

5.x release lineup evolves Robonomics into a modular infrastructure protocol for cyber-physical systems.

The architecture is built around four core protocol capabilities:

```text
Subscription
    -> global resource quota
    -> metering
    -> accounting

CPS
    -> infrastructure topology
    -> ownership
    -> authorization
    -> local resource budgets
    -> programmable state policies

Storage
    -> chain-native persistent data
    -> content commitments
    -> retention

Compute
    -> chain-coordinated verifiable execution
    -> proofs
    -> verified state transitions
```

The core execution model is:

```text
Storage records facts.

Compute proves conditions over facts.

Policy converts proven conditions into CPS state.

Runtime commits the transition.
```

---

# 1. Runtime-first Architecture

Robonomics 5.0 moves toward a runtime-first architecture.

The core repository focuses on:

- Robonomics runtime;
- FRAME pallets;
- protocol state transitions;
- CPS topology and state;
- resource accounting;
- Storage coordination;
- Compute coordination;
- programmable policies;
- runtime upgrades.

Operational tooling remains outside the protocol core.

```text
Robonomics 50.x
    runtime
    pallets
    protocol

Robins 5.x
    node tooling
    operators
    workers
    developer utilities
```

Using `polkadot-omni-node` allows Robonomics to focus primarily on runtime and protocol development.

---

# 2. Community Governance

Governance controls protocol evolution and global infrastructure parameters.

Governance may control:

```text
runtime upgrades

subscription economics

Storage parameters

Compute parameters

retention policy

global resource limits

proof verification policy

registered Compute classes

emergency controls
```

As Storage and Compute become protocol resources, governance controls not only blockchain parameters but also important constraints of the infrastructure network.

---

# 3. Subscription as the Global Resource Layer

Subscription is the global resource accounting layer.

```text
Account
    ->
Subscription
```

No additional subscription delegation registry is required.

A Subscription defines the owner's global resource pool:

```text
Subscription
|
+-- Transactions
|
+-- Storage
|
`-- Compute
```

One owner may have multiple CPS trees consuming the same Subscription:

```text
Owner Subscription
        |
        +-- CPS Tree A
        |
        +-- CPS Tree B
        |
        `-- CPS Tree C
```

Subscription does not define authorization.

It answers:

> How much resource does the owner have, how much has already been consumed, and can another operation be accepted?

---

# 4. CPS Node as Execution Context

A CPS node is not a Storage container and not an executable actor.

It is a stable **semantic, resource and execution context**.

Conceptually:

```text
CPS Node
|
+-- identity
+-- parent
+-- owner
+-- meta
+-- payload
`-- attributes
```

A node answers:

```text
WHAT object is this?

WHO owns it?

WHO may act on it?

WHICH resource domain does it belong to?

WHAT is its current state?

WHICH verified policies constrain its transitions?
```

Storage, Compute and future protocol capabilities operate **in the context of a CPS node**.

---

# 5. Meta and Payload

CPS keeps a small amount of canonical node state.

## Meta

`meta` describes what the node is and how it should be interpreted.

Examples:

```text
device type

schema

model

firmware description

calibration

configuration

content reference
```

Semantically:

```text
meta
= durable context
= description
= configuration
= interpretation rules
```

## Payload

`payload` represents the current operational state of the node.

Examples:

```text
status

mode

latest value

latest Storage CID

latest Compute result

current certification state
```

Semantically:

```text
payload
= current compact state
```

Historical or large data should not accumulate inside `payload`.

---

# 6. Node Values

Both `meta` and `payload` should support small inline values and content-addressed references.

Conceptually:

```text
Value =
    Inline(codec, bytes)

    OR

    Content(cid)
```

Example:

```text
payload =
Inline {
    status: "online"
}
```

or:

```text
payload =
Content {
    cid: bafy...
}
```

This keeps CPS compact while allowing arbitrary larger structures to live in Storage.

---

# 7. CPS Attributes

Attributes define behavior and policy attached to a node.

The base attribute model is:

```text
Attribute
|
+-- Access
|
+-- ResourceBudget
|
`-- Policy
```

Conceptually:

```text
AccessAttribute {
    principal,
    capability,
    scope
}
```

```text
ResourceBudget {
    resource,
    limit
}
```

```text
Policy {
    trigger,
    action,
    image_id
}
```

Attributes are declarative.

Mutable resource accounting remains outside CPS.

---

# 8. Capabilities

Capabilities should describe concrete operations rather than a generic unrestricted `Write`.

Initial capabilities may include:

```text
WriteMeta

WritePayload

Storage

Compute
```

Example:

```text
AccessAttribute(
    Gateway,
    WritePayload,
    Subtree
)
```

```text
AccessAttribute(
    Camera,
    Storage,
    Node
)
```

```text
AccessAttribute(
    Analytics,
    Compute,
    Subtree
)
```

The owner retains implicit administrative control.

---

# 9. Access Scope

Access can apply to:

```text
Node

Subtree
```

Example:

```text
Factory
|
+-- AccessAttribute(
|       Gateway,
|       Storage,
|       Subtree
|   )
|
+-- Line A
|   `-- Sensor A
|
`-- Line B
    `-- Sensor B
```

One attribute grants the Gateway Storage capability throughout the Factory subtree.

---

# 10. ResourceBudget

Resource limits are independent of identity.

```text
ResourceBudget {
    resource,
    limit
}
```

A budget attached to a node applies to that node's subtree.

Example:

```text
Factory
|
+-- ResourceBudget(Storage, 600 GB)
|
+-- AccessAttribute(Gateway A, Storage, Subtree)
|
+-- AccessAttribute(Gateway B, Storage, Subtree)
|
+-- Line A
|
`-- Line B
```

All authorized consumers share the same resource domain:

```text
Gateway A
+ Gateway B
+ other consumers
<= 600 GB
```

Root budgets and subtree budgets therefore use the same mechanism:

```text
ResourceBudget(node, resource, limit)
```

The CPS hierarchy defines the resource domains.

---

# 11. Resource Accounting

CPS contains declarative resource policy.

Subscription contains mutable accounting state.

```text
CPS
=
authorization
+ ResourceBudget


Subscription
=
global quota
+ metering
+ usage counters
```

The base accounting model is:

```text
OwnerUsage(owner, resource)

BudgetUsage(node, resource)
```

A node becomes an accounting boundary only when it contains a corresponding `ResourceBudget`.

For a target node:

```text
Line budget
AND
Factory budget
AND
Company budget
AND
Owner Subscription quota
```

must all allow the operation.

---

# Storage

# 12. Storage as a State Transition

Storage is a chain-native resource-consuming state transition.

Conceptually:

```text
Storage::store(
    cps_node,
    data
)
```

The transition performs:

```text
CPS authorization
        |
        v
resolve owner
        |
        v
resolve ResourceBudgets
        |
        v
Subscription accounting
        |
        v
content commitment
        |
        v
transaction storage
```

Storage either succeeds atomically or fails.

---

# 13. Storage State vs Bulk Data

Bulk bytes do not belong in ordinary FRAME state.

```text
Runtime State
|
+-- CPS topology
+-- meta / payload
+-- permissions
+-- ResourceBudgets
+-- Subscription accounting
+-- content commitments
+-- Storage metadata
`-- retention state


Node Storage
|
+-- transaction payloads
+-- chunks
+-- manifests
`-- bulk data


Network Retrieval
|
+-- Bitswap
+-- IPFS
`-- node APIs
```

The principle is:

> Storage is protocol state, while bulk persistence is delegated to specialized node storage.

---

# 14. Storage and CPS

A Storage object is associated with a CPS execution context.

```text
Storage::store(
    node = Sensor,
    data = telemetry
)
```

The CPS node determines:

```text
authorization

resource owner

ResourceBudgets

semantic context
```

The Storage operation does **not** automatically modify `payload`.

For example:

```text
Sensor
|
+-- payload {
|       status: Online
|   }
|
`-- Storage
    +-- CID-1
    +-- CID-2
    +-- CID-3
    `-- CID-4
```

Applications may optionally update:

```text
payload.latest = CID-4
```

through a separate authorized or policy-driven transition.

---

# 15. Storage Lifecycle

Storage is a lifecycle:

```text
store
  ->
commit
  ->
persist
  ->
retrieve
  ->
prove availability
  ->
renew
  ->
expire
```

Retention can be maintained while:

```text
Subscription is active

AND

Storage quota is available

AND

applicable ResourceBudgets allow it
```

Automatic renewal should be infrastructure behavior rather than a responsibility of every sensor application.

---

# 16. Large Objects

Large objects should be chunked by SDK tooling:

```text
Large Object
     |
     v
SDK chunking
     |
     +-- Chunk A -> Storage::store()
     |
     +-- Chunk B -> Storage::store()
     |
     `-- Chunk N -> Storage::store()
             |
             v
          Manifest
             |
             v
            CID
```

The chain primitive remains small and deterministic.

---

# Compute

# 17. Compute as a State Transition

Compute follows the same protocol architecture as Storage.

The computation itself does not execute inside the runtime.

The runtime coordinates and commits verifiable work.

```text
Compute::request(...)
        |
        v
Runtime Transition
        |
        +-- CPS authorization
        +-- ResourceBudget checks
        +-- Subscription accounting
        `-- ComputeJob
                 |
                 v
          external worker
                 |
                 v
            RISC Zero
                 |
                 v
              Receipt
                 |
                 v
        Compute::submit(...)
                 |
                 v
        runtime commitment
```

---

# 18. CPS Node as Compute Context

Every Compute job belongs to a CPS node.

```text
ComputeJob {
    node,
    image_id,
    inputs,
    state_snapshot,
    status
}
```

The node determines:

```text
WHO may request Compute

WHO pays

WHICH ResourceBudgets apply

WHICH current CPS state belongs to the job

WHAT semantic object the result belongs to
```

The Compute worker itself does not need to belong to the user's CPS tree.

---

# 19. Compute Inputs

Compute may consume three classes of input:

```text
CPS meta

CPS payload

Storage content
```

Example:

```text
             Sensor
            /      \
         meta      payload
           \         /
            \       /
          snapshot
              |
      Storage input CID
              |
              v
       Compute::request
              |
              v
          RISC Zero
```

`meta` provides interpretation and configuration.

`payload` provides current operational state.

Storage provides large or historical datasets.

---

# 20. State Snapshots

Compute jobs must be bound to the exact CPS state they operate on.

Conceptually:

```text
ComputeContext {
    node,
    meta_hash,
    payload_hash,
    input_cids
}
```

A receipt or public journal should commit to:

```text
node

meta_hash

payload_hash

input commitment

result
```

This prevents an old proof from silently applying to newer CPS state.

---

# 21. Compute Result

Compute produces a verified fact, not an automatic arbitrary CPS mutation.

Conceptually:

```text
ComputeResult {
    job_id,
    node,
    image_id,
    state_snapshot,
    journal,
    output_cid,
    receipt
}
```

A result may be used in two ways.

## Data-only result

```text
Storage inputs
      |
      v
Compute
      |
      v
Storage output
```

CPS does not change.

## State-changing result

```text
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

---

# Programmable CPS Policies

# 22. Policy Model

Policies define verified conditions under which CPS actions or state transitions may occur.

Conceptually:

```text
Policy {
    trigger,
    action,
    image_id
}
```

Possible triggers:

```text
StorageEvent

ComputeResult

ExplicitRequest

Timer

ExternalEvent
```

Possible actions:

```text
UpdatePayload

UpdateMeta

TransitionState

GrantDerivedPermission

EmitAction
```

The first implementation should keep the supported trigger and action set intentionally small.

---

# 23. Proof-gated State Transitions

A CPS state transition may require proof that a specific RISC Zero program accepted it.

Instead of:

```text
authorized caller
    ->
state change
```

Robonomics can support:

```text
authorized caller
        +
verified computation
        ->
state change
```

Example:

```text
Robot
|
+-- payload {
|       state: Maintenance
|   }
|
`-- Policy {
        trigger: ComputeResult,
        action: SetOperational,
        image_id: DiagnosticsPolicyV1
    }
```

Transition:

```text
Diagnostics data
       |
       v
RISC Zero
       |
       v
Receipt
       |
       v
Policy verification
       |
       v
Maintenance -> Operational
```

---

# 24. Automatic State Updates

Policies may automatically update CPS state after a verified Compute result.

Example:

```text
Storage::store(Sensor, telemetry)
        |
        v
Storage event
        |
        v
Policy matched
        |
        v
Compute::request(
    node = Sensor,
    image_id = TemperaturePolicy
)
        |
        v
Receipt
        |
        v
verified transition
        |
        v
payload.status = Alarm
```

The resulting model is:

```text
Storage
    creates facts

Compute
    proves conditions

Policy
    interprets the proof

CPS
    commits new state
```

---

# 25. Safe Automatic Transitions

A Compute proof must be bound to the previous CPS state.

Conceptually:

```text
PolicyResult {
    node,
    previous_state_hash,
    action,
    new_state_commitment
}
```

Before applying the result:

```text
receipt is valid

AND

image_id matches Policy

AND

node matches

AND

previous_state_hash == current_state_hash

AND

action is allowed by Policy
```

If the CPS state changed while computation was running:

```text
State S1
  |
  +--> Compute starts
  |
  `--> CPS changes to S2

Receipt for S1 arrives
```

the transition is rejected.

This creates an optimistic but safe verified-state-transition model.

---

# 26. Manual and Automatic Policies

Policies may support two application modes.

## Manual

```text
Compute result
    ->
explicit apply
    ->
CPS transition
```

Useful when human or application approval is required.

## Automatic

```text
Compute result
    ->
Policy verification
    ->
CPS transition
```

Useful for deterministic autonomous systems.

---

# 27. Policy Examples

## Telemetry State

```text
temperature data
    ->
RISC Zero
    ->
temperature > threshold
    ->
payload.status = Alarm
```

## Device Certification

```text
telemetry + diagnostics
    ->
compliance policy
    ->
Receipt
    ->
payload.certified = true
```

## Firmware Upgrade

```text
device state
+ firmware hash
+ approved manifest
        |
        v
RISC Zero
        |
        v
upgrade allowed
```

## Robot State

```text
signed diagnostics
        |
        v
Diagnostics Policy
        |
        v
Receipt
        |
        v
Maintenance -> Operational
```

## Private Compliance

```text
private dataset
        |
        v
RISC Zero
        |
        v
compliant = true
```

Raw input does not need to become public.

---

# 28. Unified Storage / Compute / CPS Model

The complete lifecycle becomes:

```text
Physical Device
      |
      v
Storage::store(node, data)
      |
      v
Content CID
      |
      +------------------+
      |                  |
      |                  v
      |          Compute::request()
      |                  |
      |                  v
      |             RISC Zero
      |                  |
      |                  v
      |               Receipt
      |                  |
      |                  v
      |               Policy
      |                  |
      |                  v
      +------------> CPS state
```

A CPS node therefore connects:

```text
identity

current state

historical data

resource ownership

authorization

verifiable computation

state-transition policy
```

without becoming a bulk data store or compute worker itself.

---

# 29. Storage and Compute Symmetry

Storage and Compute follow the same resource-backed protocol pattern.

```text
                  State Transitions
                         |
          +--------------+--------------+
          |                             |
          v                             v
   Storage::store()              Compute::request()
          |                             |
          v                             v
   transaction storage             Compute worker
          |                             |
          v                             v
      CID / proof                  zkVM Receipt
          |                             |
          +--------------+--------------+
                         |
                         v
                    CPS State
```

The runtime owns:

```text
authorization

accounting

commitments

job state

policy state

canonical transitions
```

Specialized infrastructure performs:

```text
bulk persistence

heavy computation

proof generation
```

---

# 30. Protocol Layer vs Execution Layer

## Protocol / State Transition Layer

```text
Governance

Subscription

CPS

Storage protocol

Compute coordination

Policies
```

This layer determines:

```text
what is allowed

who may request it

who pays

which budgets apply

what becomes canonical state
```

## Node / Execution / Persistence Layer

```text
transaction storage

content-addressed persistence

Bitswap / IPFS

Compute workers

RISC Zero executors

RISC Zero provers
```

This layer performs expensive physical work.

---

# 31. Verifiable CPS Data Pipeline

Storage and Compute compose into a generic verifiable data pipeline.

```text
Signed Sensor
      |
      v
Storage::store()
      |
      v
Raw Data CID
      |
      v
Compute::request()
      |
      v
RISC Zero
      |
      +--> Receipt
      |
      `--> Derived Data
              |
              v
        Storage::store()
              |
              v
         Result CID
              |
              v
            Policy
              |
              v
         CPS State
```

Possible workloads include:

```text
aggregation

filtering

calibration

compliance

anomaly detection

scientific processing

robotics planning

data transformation

cryptographic validation

ML inference where practical
```

---

# 32. sensors.social

`sensors.social` becomes an initial end-to-end application.

```text
Sensor
    ->
CPS registration
    ->
Storage authorization
    ->
Storage::store()
    ->
telemetry CID
    ->
optional Compute
    ->
verified result
    ->
optional Policy transition
```

For example:

```text
Environmental Sensor
        |
        v
raw measurements
        |
        v
Storage
        |
        v
RISC Zero aggregation
        |
        +--> Receipt
        |
        v
daily dataset
        |
        v
Storage
        |
        v
CPS payload.latest_result
```

---

# 33. Target Architecture

```text
+--------------------------------------------------+
|              Robonomics Runtime                  |
|                                                  |
| Governance                                       |
|                                                  |
| Subscription                                     |
|   global quotas                                  |
|   usage accounting                               |
|                                                  |
| CPS                                              |
|   topology                                       |
|   ownership                                      |
|   meta                                           |
|   payload                                        |
|   AccessAttributes                               |
|   ResourceBudgets                                |
|   Policies                                       |
|                                                  |
| Storage                                          |
|   state transitions                              |
|   commitments                                    |
|   retention                                      |
|                                                  |
| Compute                                          |
|   jobs                                           |
|   receipts                                       |
|   verified results                               |
+-------------------------+------------------------+
                          |
                          v
+--------------------------------------------------+
|          Node / Execution Infrastructure         |
|                                                  |
| transaction storage                              |
| content-addressed persistence                    |
| Bitswap / IPFS                                   |
|                                                  |
| RISC Zero workers                                |
| executors                                        |
| provers                                          |
+-------------------------+------------------------+
                          |
                          v
+--------------------------------------------------+
|                  Applications                    |
|                                                  |
| sensors.social                                   |
| robotics                                         |
| scientific networks                              |
| industrial IoT                                   |
| autonomous systems                               |
+--------------------------------------------------+
```

---

# 34. Implementation Sequence

## Phase 1 - Runtime Foundation

```text
1. Runtime separation

2. Governance activation

3. Subscription v2
   + generic resource accounting
```

## Phase 2 - CPS Core

```text
4. CPS execution-context model

5. meta / payload value model

6. AccessAttribute

7. ResourceBudget

8. CPS <-> Subscription accounting interface
```

## Phase 3 - Storage

```text
9. Adapt Bulletin transaction-storage primitives

10. Storage::store(cps_node, data)

11. CPS Storage authorization

12. Subscription Storage accounting

13. Content addressing

14. Chunking and manifests

15. Retention and renewal

16. Availability lifecycle
```

## Phase 4 - Compute

```text
17. RISC Zero prototype

18. ComputeJob model

19. CPS state snapshots

20. Compute::request()

21. Worker / prover infrastructure

22. Compute::submit()

23. Receipt verification

24. Compute accounting
```

## Phase 5 - Programmable CPS

```text
25. Policy model

26. Policy ImageID references

27. Proof-gated transitions

28. Automatic payload updates

29. State-version / stale-proof protection

30. Manual and automatic application modes
```

## Phase 6 - Applications

```text
31. Storage + Compute pipeline

32. sensors.social migration

33. robotics policies

34. industrial / scientific use cases

35. additional protocol capabilities
```

---

# 35. Architectural Principles

## CPS Node

```text
A CPS node is a stable semantic,
resource and execution context.
```

It does not need to physically store large data or execute heavy computation.

## Meta

```text
meta
-> WHAT the node is
-> HOW it should be interpreted
```

## Payload

```text
payload
-> WHAT the node's current state is
```

## AccessAttribute

```text
AccessAttribute
-> WHO may act
-> WHAT they may do
-> WHERE they may act
```

## ResourceBudget

```text
ResourceBudget
-> HOW MUCH a subtree may consume
```

## Subscription

```text
Subscription
-> HOW MUCH resource the owner has
-> HOW MUCH has already been consumed
```

## Storage

```text
Storage
-> RECORDS facts
-> PERSISTS bulk data
```

## Compute

```text
Compute
-> DERIVES verifiable facts
-> PROVES computation
```

## Policy

```text
Policy
-> determines WHEN a proven fact
   may change canonical CPS state
```

---

# 36. Robonomics 5.0 Vision

Robonomics becomes a protocol for coordinating physical infrastructure, persistent data and verifiable computation.

```text
                 Governance
                     |
                     v
          +----------+----------+
          |                     |
          v                     v
         CPS               Subscription
          |                     |
 topology/state             quota/accounting
 authorization
 budgets
 policies
          |                     |
          +----------+----------+
                     |
                     v
              State Transitions
                     |
          +----------+----------+
          |                     |
          v                     v
       Storage                Compute
          |                     |
          v                     v
       Facts                 Proofs
          |                     |
          +----------+----------+
                     |
                     v
                  Policy
                     |
                     v
                 CPS State
```

The core Robonomics 5.0 model is:

> **Owner pays.**  
> **Delegate acts.**  
> **CPS provides context and authorization.**  
> **Subscription accounts.**  
> **Storage records facts.**  
> **Compute proves conditions.**  
> **Policy advances state.**  
> **Runtime commits.**

Robonomics 5.0 is therefore not a blockchain connected to external storage and compute services.

It is a protocol where **persistent data, verifiable computation and programmable CPS state transitions are first-class resource-backed state transitions**.
