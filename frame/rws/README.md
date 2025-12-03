# RWS Pallet 2.0 - Technical Guide

## Overview

The Robonomics Web Services (RWS) Pallet provides a decentralized auction system for obtaining subscriptions that enable free transaction execution on the Robonomics Network. Users bid on subscriptions, and winning bidders receive the ability to execute transactions without paying fees, metered by computational weight and time.

---

## 📊 Subscription Types

### Lifetime Subscription
```
┌─────────────────────────────────────────┐
│  Lifetime { tps: u32 }                  │
│  ═══════════════════════════════════    │
│                                         │
│  • Custom TPS allocation                │
│  • Never expires                        │
│  • Measured in μTPS (micro-TPS)         │
│  • Example: { tps: 500_000 }            │
│    = 0.5 transactions per second        │
└─────────────────────────────────────────┘
```

### Daily Subscription
```
┌─────────────────────────────────────────┐
│  Daily { days: u32 }                    │
│  ═══════════════════════════════════    │
│                                         │
│  • Fixed rate: 0.01 TPS (10,000 μTPS)   │
│  • Time-limited validity                │
│  • Expires after N days                 │
│  • Example: { days: 30 }                │
│    = 30 days of service                 │
└─────────────────────────────────────────┘
```

---

## 🎯 Auction Lifecycle

```
╔════════════════════════════════════════════════════════════════════╗
║                     COMPLETE AUCTION FLOW                          ║
╚════════════════════════════════════════════════════════════════════╝

PHASE 1: AUCTION CREATION
━━━━━━━━━━━━━━━━━━━━━━━━
┌──────────────┐
│ Governance   │
│ (Root)       │
└──────┬───────┘
       │ start_auction(SubscriptionMode)
       │
       ▼
┌──────────────────────────────┐
│ New Auction Created          │
│ • ID assigned (auto-inc)     │
│ • Type: Daily or Lifetime    │
│ • Status: OPEN               │
│ • No winner yet              │
└──────────────────────────────┘
       │
       │ Emits: AuctionStarted(id)
       ▼


PHASE 2: BIDDING PERIOD
━━━━━━━━━━━━━━━━━━━━━━━━
┌─────────┐  ┌─────────┐  ┌─────────┐
│ User A  │  │ User B  │  │ User C  │
│ Bid: 100│  │ Bid: 150│  │ Bid: 200│
└────┬────┘  └────┬────┘  └────┬────┘
     │            │            │
     └────────────┼────────────┘
                  │
                  ▼
      ┌───────────────────────────────┐
      │  Auction State                │
      │  ─────────────────────────    │
      │  winner: User C               │
      │  best_price: 200              │
      │  first_bid_time: T₀           │
      │  funds: RESERVED              │
      └───────────────────────────────┘
                  │
                  │ Previous bids unreserved
                  │ Current bid reserved
                  ▼
      Each bid must exceed:
      • MinimalBid (first bid)
      • Current best_price (subsequent)

      ⏰ Bidding Period: first_bid_time + AuctionDuration
      🔒 After period ends: No more bids accepted


PHASE 3: CLAIM & FINALIZATION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Winner calls claim() after bidding period ends:

┌────────────────────────────────────┐
│ claim(auction_id, beneficiary?)    │
└────────────────────────────────────┘
              │
              ▼
    ┌─────────────────────┐
    │ Validation Checks   │
    │ • Is winner?        │
    │ • Period ended?     │
    │ • Not claimed yet?  │
    └─────────────────────┘
              │
              ▼
┌───────────────────────────────────┐
│ Winner's Funds Processing         │
│ • Reserved amount SLASHED         │
│ • Tokens BURNED                   │
└───────────────────────────────────┘
              │
              ▼
┌───────────────────────────────────┐
│ Subscription Created              │
│ • subscription_id assigned (0,1. .)│
│ • Stored in Subscription storage  │
│ • Owner: beneficiary or winner    │
└───────────────────────────────────┘
              │
              │ Emits: AuctionFinished(auction_id)
              │        SubscriptionActivated(owner, sub_id)
              ▼
        [ACTIVE SUBSCRIPTION]


PHASE 4: USAGE
━━━━━━━━━━━━━━━
┌────────────────────────────────────┐
│ Subscription Owner                 │
│ └─▶ call(subscription_id, call)   │
└────────────────────────────────────┘
              │
              ▼
    ┌─────────────────────┐
    │ Weight Accumulation │
    │ & Expiration Check  │
    └─────────────────────┘
              │
         ┌────┴─────┐
         ▼          ▼
    [APPROVED]  [REJECTED]
         │          │
         │          └─▶ Error: FreeWeightIsNotEnough
         │              or SubscriptionIsOver
         │
         ▼
    Execute with Pays::No
    Deduct call weight
```

---

## 💡 Free Weight Mechanism

The system uses **weight-based metering** to control transaction throughput:

```
WEIGHT ACCUMULATION FORMULA
═══════════════════════════

free_weight += ReferenceCallWeight × (μTPS) × Δt_seconds
                                            ─────────────
                                             1,000,000,000

Where:
• ReferenceCallWeight = Weight of standard transaction
• μTPS = Micro-TPS (tps for Lifetime, 10,000 for active Daily)
• Δt = Time since last_update (in seconds)


TIMELINE EXAMPLE (Lifetime: 500,000 μTPS = 0.5 TPS)
═══════════════════════════════════════════════════

T₀: Subscription Created
    ┌────────────────┐
    │ free_weight: 0 │
    └────────────────┘

T₀ + 1s
    ┌──────────────────────────┐
    │ free_weight: 35,476,000  │  (1 call worth)
    └──────────────────────────┘

T₀ + 2s
    ┌──────────────────────────┐
    │ free_weight: 70,952,000  │  (2 calls worth)
    └──────────────────────────┘

T₀ + 2s: User executes call
    ┌──────────────────────────┐
    │ free_weight: 35,476,000  │  (1 call remaining)
    └──────────────────────────┘

T₀ + 10s
    ┌───────────────────────────┐
    │ free_weight: 354,760,000  │  (10 calls worth)
    └───────────────────────────┘


DAILY SUBSCRIPTION BEHAVIOR
════════════════════════════

┌─────────────────────────────────────────┐
│ Cached expiration_time on creation:     │
│ issue_time + (days × 86400s)            │
└─────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│ While: now < expiration_time            │
│ TPS = 10,000 μTPS (0.01 TPS)            │
└─────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│ After expiration:                       │
│ Error: SubscriptionIsOver               │
│ No calls allowed                        │
└─────────────────────────────────────────┘
```

---

## 🏗️ Data Structures

```
STORAGE LAYOUT
══════════════

Auction: CountedStorageMap<u32, AuctionLedger>
├─ Auction(0) ─┐
│              ▼
│        AuctionLedger {
│          winner: Some(Alice),
│          best_price: 200,
│          first_bid_time: Some(T₀),
│          mode: Daily { days: 30 },
│          subscription_id: None  ← Not claimed yet
│        }
│
└─ Auction(1) ─┐
               ▼
         AuctionLedger {
           winner: Some(Bob),
           best_price: 150,
           first_bid_time: Some(T₁),
           mode: Lifetime { tps: 100_000 },
           subscription_id: Some(0)  ← Claimed!
         }


Subscription: DoubleMap<AccountId, u32, SubscriptionLedger>
├─ Subscription(Alice, 0) ─┐
│                          ▼
│                    SubscriptionLedger {
│                      free_weight: 1_000_000,
│                      issue_time: 1735401600000,
│                      last_update: 1735401700000,
│                      mode: Daily { days: 30 },
│                      expiration_time: Some(1738080000000)
│                    }
│
├─ Subscription(Alice, 1) ─┐  ← Multiple subs per account! 
│                          ▼
│                    SubscriptionLedger {
│                      free_weight: 500_000,
│                      issue_time: 1735401500000,
│                      last_update: 1735401500000,
│                      mode: Lifetime { tps: 500_000 },
│                      expiration_time: None
│                    }
│
└─ Subscription(Bob, 0) ───┐
                           ▼
                     SubscriptionLedger { ... }
```

---

## 🔑 Extrinsics Reference

### User Functions

#### `bid(auction_id, amount)`
Place a bid on an active auction. 

**Requirements:**
- Auction must exist
- Amount > `MinimalBid` (first bid) OR amount > current `best_price`
- Bidding period not ended (first_bid_time + AuctionDuration)
- Sufficient balance for reservation

**Effects:**
- Reserves bid amount from caller
- Unreserves previous winner's amount
- Updates auction winner and price
- Sets `first_bid_time` on first bid

---

#### `claim(auction_id, beneficiary? )`
Claim a won auction and activate subscription.

**Requirements:**
- Caller must be auction winner
- Bidding period must be ended
- Auction not already claimed
- At least one bid placed

**Parameters:**
- `beneficiary: Option<AccountId>` - Optional recipient (defaults to caller)

**Effects:**
- Burns winner's reserved funds
- Creates new subscription for beneficiary
- Assigns subscription_id (incremental per account)
- Marks auction as claimed

---

#### `call(subscription_id, call)`
Execute a free transaction using a subscription.

**Requirements:**
- Caller must own the subscription (indexed by caller + subscription_id)
- Subscription must not be expired (for Daily mode)
- Sufficient `free_weight` accumulated

**Effects:**
- Updates `free_weight` (accumulates then deducts)
- Executes `call` with `Pays::No`
- Updates `last_update` timestamp

---

### Governance Functions

#### `start_auction(mode)`
Create a new subscription auction.

**Origin:** Root

**Parameters:**
- `mode: SubscriptionMode` - Type of subscription (Daily or Lifetime)

**Effects:**
- Auto-increments auction counter
- Creates empty `AuctionLedger`
- Auction immediately available for bidding

---

## ⚙️ Configuration Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `AuctionDuration` | `Moment` | Duration of bidding period (in milliseconds) |
| `MinimalBid` | `Balance` | Minimum first bid amount |
| `ReferenceCallWeight` | `u64` | Weight unit for TPS calculations |

---

## 📡 Events

| Event | Parameters | Description |
|-------|------------|-------------|
| `AuctionStarted` | `(u32)` | New auction created |
| `NewBid` | `(u32, AccountId, Balance)` | Bid placed |
| `AuctionFinished` | `(u32)` | Auction claimed |
| `SubscriptionActivated` | `(AccountId, u32)` | Subscription activated for user |
| `RwsCall` | `(AccountId, u32, DispatchResult)` | Free transaction executed |

---

## 🚀 Usage Example

```rust
// STEP 1: Governance starts auction for 30-day subscription
start_auction(Daily { days: 30 })
// → Auction #0 created
// → Emits: AuctionStarted(0)

// STEP 2: Alice bids 100 XRT
bid(auction_id: 0, amount: 100_000_000_000)
// → Alice is winning, 100 XRT reserved
// → first_bid_time set to now
// → Bidding period: now + AuctionDuration

// STEP 3: Bob outbids with 150 XRT
bid(auction_id: 0, amount: 150_000_000_000)
// → Bob is now winning, 150 XRT reserved
// → Alice's 100 XRT unreserved

// STEP 4: Wait for AuctionDuration to pass

// STEP 5: Bob claims the auction
claim(auction_id: 0, beneficiary: None)
// → Bob's 150 XRT burned
// → Subscription #0 created for Bob
// → Emits: AuctionFinished(0)
// → Emits: SubscriptionActivated(Bob, 0)

// STEP 6: Bob uses his subscription
call(
    subscription_id: 0,
    call: datalog::record(b"temperature:23.5C")
)
// → Transaction executes with Pays::No
// → No fees charged
// → free_weight deducted

// STEP 7: Bob can claim more auctions
// Each creates a new subscription_id (0, 1, 2, ...)
```

---

## 🔐 Proxy-Based Subscription Sharing

The RWS pallet integrates with `pallet-proxy` to enable **subscription sharing**. This allows subscription owners to delegate usage of their subscriptions to other accounts without transferring ownership.

### Overview

The `ProxyType::RwsUser(subscription_id)` variant allows controlled delegation:
- **Single Purpose**: Only allows using a specific subscription via `RWS::call`
- **No Management Access**: Proxies cannot bid on auctions, claim subscriptions, or perform other management operations
- **Subscription-Specific**: Each proxy is limited to a single subscription ID
- **Revocable**: The subscription owner can remove proxy access at any time

Integration with `pallet-proxy` provides:
- ✅ **Type Safety**: `RwsUser` restricts to subscription usage only
- ✅ **Subscription-Specific**: Each proxy targets exactly one subscription
- ✅ **Ownership Preservation**: Original owner retains full control
- ✅ **Revocability**: Proxies can be removed at any time
- ✅ **Auditability**: All proxy actions are traceable on-chain
- ✅ **No Privilege Escalation**: Proxies cannot grant additional permissions

### ProxyType Configuration

The runtime defines a `ProxyType::RwsUser` variant for subscription sharing:

```rust
pub enum ProxyType {
    /// Allow all calls
    Any,
    /// RWS subscription user - allows using a specific subscription via RWS::call
    /// The parameter is the subscription_id that the proxy can use
    RwsUser(u32),
}

impl frame_support::traits::InstanceFilter<RuntimeCall> for ProxyType {
    fn filter(&self, c: &RuntimeCall) -> bool {
        match self {
            ProxyType::Any => true,
            ProxyType::RwsUser(allowed_subscription_id) => {
                // Only allow RWS::call operations for the specific subscription
                match c {
                    RuntimeCall::RWS(pallet_rws::Call::call { subscription_id, .. }) => {
                        subscription_id == allowed_subscription_id
                    }
                    _ => false,
                }
            }
        }
    }
    
    fn is_superset(&self, o: &Self) -> bool {
        match (self, o) {
            (ProxyType::Any, _) => true,
            (_, ProxyType::Any) => false,
            (ProxyType::RwsUser(a), ProxyType::RwsUser(b)) => a == b,
        }
    }
}
```

### Complete User Story: IoT Device Access

This example demonstrates how Alice shares her subscription with an IoT device.

```rust
// ═══════════════════════════════════════════════════════════════════
// SCENARIO: Alice owns a subscription and wants to share it with an
// IoT device (represented by BOB's account) so the device can execute
// transactions using Alice's subscription.
// ═══════════════════════════════════════════════════════════════════

// STEP 1: Alice acquires a subscription
// ───────────────────────────────────────────────
Timestamp::set_timestamp(1_000_000);

// Root starts auction for lifetime subscription
RWS::start_auction(
    RuntimeOrigin::root(),
    SubscriptionMode::Lifetime { tps: 1_000_000 }  // 1 TPS
)?;

// Alice bids and wins
RWS::bid(RuntimeOrigin::signed(ALICE), 0, 100 * XRT)?;
// → Alice is winning bidder
// → 100 XRT reserved from Alice's balance

// Wait for auction to end
Timestamp::set_timestamp(1_000_000 + AuctionDuration::get() + 1);

// Alice claims the subscription
RWS::claim(RuntimeOrigin::signed(ALICE), 0, None)?;
// → Subscription 0 created for Alice
// → 100 XRT burned
// → Alice can now use subscription for free transactions

// STEP 2: Alice adds IoT device as RwsUser proxy
// ────────────────────────────────────────────────
const IOT_DEVICE: AccountId = BOB; // Device account

// Alice creates a proxy for the IoT device with RwsUser type for subscription 0
Proxy::add_proxy(
    RuntimeOrigin::signed(ALICE),
    IOT_DEVICE,
    ProxyType::RwsUser(0),  // Can only use subscription 0
    0  // No announcement delay
)?;
// → Device can now use Alice's subscription 0
// → Device CANNOT bid on auctions, claim subscriptions, or use other subscriptions
// → Deposit reserved from Alice for proxy storage

// STEP 3: IoT device uses subscription on Alice's behalf
// ────────────────────────────────────────────────────────
Timestamp::set_timestamp(2_000_000);

// Device wants to record sensor data using Alice's subscription
let sensor_data = RuntimeCall::Datalog(
    pallet_datalog::Call::record {
        record: b"temperature:23.5C".to_vec().try_into().unwrap()
    }
);

// Device executes via proxy
Proxy::proxy(
    RuntimeOrigin::signed(IOT_DEVICE),
    ALICE,  // Real account (subscription owner)
    Some(ProxyType::RwsUser(0)),
    Box::new(RuntimeCall::RWS(
        pallet_rws::Call::call {
            subscription_id: 0,
            call: Box::new(sensor_data)
        }
    ))
)?;
// → Device successfully uses Alice's subscription
// → Transaction executes with Pays::No (no fees)
// → Alice's subscription weight is deducted
// → Sensor data recorded to chain

// STEP 4: Alice can revoke access when needed
// ───────────────────────────────────────────────
// If device is compromised or decommissioned, Alice revokes access
Proxy::remove_proxy(
    RuntimeOrigin::signed(ALICE),
    IOT_DEVICE,
    ProxyType::RwsUser(0),
    0
)?;
// → Device can no longer use Alice's subscription
// → Alice's deposit returned
// → Subscription remains active for Alice

// STEP 5: Alice retains full control
// ──────────────────────────────────────────────────
RWS::call(
    RuntimeOrigin::signed(ALICE),
    0,  // subscription_id
    Box::new(RuntimeCall::Datalog(
        pallet_datalog::Call::record {
            record: b"manual_entry:data".to_vec().try_into().unwrap()
        }
    ))
)?;
// → Alice retains full control regardless of proxy status
```

### Additional Usage Example: Multisig Shared Subscription

Combine with multisig for team-managed subscription sharing:

```rust
// Team creates a multisig account and acquires subscription
let team_account = TEAM_MULTISIG;

// Team multisig wins auction and claims subscription
RWS::bid(RuntimeOrigin::signed(TEAM_MULTISIG), 0, 200 * XRT)?;
// ... auction ends ...
RWS::claim(RuntimeOrigin::signed(TEAM_MULTISIG), 0, None)?;

// Team adds individual members as RwsUser proxies
Proxy::add_proxy(
    RuntimeOrigin::signed(TEAM_MULTISIG),
    ALICE,
    ProxyType::RwsUser(0),  // Alice can use team subscription 0
    0
)?;

Proxy::add_proxy(
    RuntimeOrigin::signed(TEAM_MULTISIG),
    BOB,
    ProxyType::RwsUser(0),  // Bob can also use team subscription 0
    0
)?;

// Now Alice or Bob can use the team subscription independently
Proxy::proxy(
    RuntimeOrigin::signed(ALICE),
    TEAM_MULTISIG,
    Some(ProxyType::RwsUser(0)),
    Box::new(RuntimeCall::RWS(pallet_rws::Call::call {
        subscription_id: 0,
        call: Box::new(some_transaction)
    }))
)?;
```

### Security Considerations

#### Type Safety
- **Restricted Call Space**: `ProxyType::RwsUser` only allows `RWS::call` for a specific subscription
- **No Management Operations**: Proxies cannot bid on auctions, claim subscriptions, or perform other management tasks
- **Compile-Time Guarantees**: Substrate's type system enforces restrictions

#### Subscription-Specific Control
- **Single Subscription**: Each proxy is limited to exactly one subscription ID
- **No Cross-Subscription Access**: Proxy for subscription 0 cannot access subscription 1
- **Granular Permissions**: Owner can create different proxies for different subscriptions

#### Ownership Preservation
- **Owner Supremacy**: Original account owner retains full control
- **Independent Access**: Owner can use subscription even while proxies are active
- **Proxy Removal**: Owner can revoke proxy access instantly
- **No Transfer**: Proxies do not transfer ownership; they delegate specific operations

#### Revocability
- **Instant Revocation**: `Proxy::remove_proxy` immediately blocks proxy access
- **Deposit Recovery**: Proxy deposits returned to owner upon removal
- **No Backdoors**: Revoked proxies have zero access to subscription

#### Auditability
- **On-Chain Records**: All proxy actions emit events with full context
- **Transparent Delegation**: `Proxy::proxies(account)` lists all active proxies
- **Action Attribution**: Events identify both proxy (delegate) and real (owner) accounts

#### No Privilege Escalation
- **Closed Permission Model**: Proxies cannot grant new proxies
- **Usage Only**: `RwsUser` proxies can only execute calls via subscriptions
- **Isolated Operations**: Actions limited to explicitly granted capabilities

### Use Cases

#### 1. IoT Device Fleet Management
- **Scenario**: Company with sensors needing free transactions
- **Solution**: Company account owns subscription; each sensor is a proxy
- **Benefits**: 
  - Sensors operate autonomously without exposing company keys
  - Individual sensor compromise doesn't affect others (revoke single proxy)
  - Centralized subscription management with distributed execution

#### 2. Shared Team Subscriptions
- **Scenario**: Team wants shared subscription for member activities
- **Solution**: Multisig account owns subscription; members are proxies
- **Benefits**:
  - Members can use subscription without multisig approval per transaction
  - Critical operations (new subscription, revoke member) require multisig
  - Clear audit trail of which member performed each action

#### 3. Service Account Delegation
- **Scenario**: Backend service needs to execute transactions for users
- **Solution**: Users grant service account proxy access to their subscriptions
- **Benefits**:
  - Service can operate on behalf of users
  - Users retain ownership and control
  - Users can revoke access anytime

#### 4. Temporary Access
- **Scenario**: User needs contractor to test subscription usage
- **Solution**: Grant contractor `RwsUser` proxy for limited time
- **Benefits**:
  - Contractor can test subscription usage
  - Easy revocation after engagement ends
  - No access to management operations

---

## 🔍 Key Design Features

### Account-Based Subscription Model
- **Multiple subscriptions per account**: Each user can have subscription IDs 0, 1, 2, etc.
- **DoubleMap storage**: Efficient lookup by (AccountId, subscription_id)
- **No device linking needed**: Subscription owner directly calls with their account

### Static Auction System
- **Permanent auction IDs**: Auto-incrementing counter
- **Time-based bidding periods**: Start when first bid is placed
- **Explicit claim phase**: Winner must manually claim after period ends
- **Unlimited parallel auctions**: No queue management needed

### Weight-Based Metering
- **Fair resource allocation**: Complex calls use more quota
- **Substrate-native**: Uses existing weight system
- **Prevents abuse**: Weight limits enforce fair usage

### Economic Model
- **Bid amounts are burned**: Deflationary mechanism
- **No refunds after winning**: Commitment mechanism
- **Market-driven pricing**: Competitive bidding

---

**Implementation:** See [PR #381](https://github.com/airalab/robonomics/pull/381) for technical details. 