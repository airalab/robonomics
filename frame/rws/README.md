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

## 🔐 Proxy-Based Access Delegation

The RWS pallet integrates seamlessly with `pallet-proxy` to enable **delegated subscription management**. This allows subscription owners to grant specific accounts the ability to manage their subscriptions without transferring ownership.

### Overview

Proxy-based access delegation enables powerful use cases:
- **IoT Device Management**: Grant devices permission to use subscriptions without exposing owner keys
- **Team Subscriptions**: Allow multiple team members to manage shared subscriptions
- **Automated Systems**: Enable bots to bid on auctions or manage subscriptions on behalf of owners
- **Time-Delayed Operations**: Schedule future subscription operations with announcement delays
- **Fine-Grained Control**: Restrict proxy access to specific auctions or operations

Integration with `pallet-proxy` provides:
- ✅ **Type Safety**: `RwsManager` proxy type restricts to RWS operations only
- ✅ **Auction-Level Granularity**: Optional auction ID restriction for precise control
- ✅ **Ownership Preservation**: Original owner retains full control
- ✅ **Revocability**: Proxies can be removed at any time
- ✅ **Auditability**: All proxy actions are traceable on-chain
- ✅ **No Privilege Escalation**: Proxies cannot grant additional permissions

### ProxyType Configuration

The runtime defines a `ProxyType::RwsManager` variant specifically for RWS subscription management:

```rust
pub enum ProxyType {
    /// Allow all calls
    Any,
    /// RWS subscription management with optional auction restriction
    /// - `RwsManager(None)`: Access to all RWS operations for subscriptions owned by proxied account
    /// - `RwsManager(Some(auction_id))`: Access only to specific auction's operations
    RwsManager(Option<u32>),
}

impl frame_support::traits::InstanceFilter<RuntimeCall> for ProxyType {
    fn filter(&self, c: &RuntimeCall) -> bool {
        match self {
            ProxyType::Any => true,
            ProxyType::RwsManager(allowed_auction) => {
                // Check if it's an RWS call
                let is_rws_call = matches!(
                    c,
                    RuntimeCall::RWS(pallet_rws::Call::bid { .. })
                        | RuntimeCall::RWS(pallet_rws::Call::claim { .. })
                        | RuntimeCall::RWS(pallet_rws::Call::call { .. })
                );
                
                if !is_rws_call {
                    return false;
                }
                
                // If no auction restriction, allow all RWS calls
                if allowed_auction.is_none() {
                    return true;
                }
                
                // Check if call targets the allowed auction
                match c {
                    RuntimeCall::RWS(pallet_rws::Call::bid { auction_id, .. }) |
                    RuntimeCall::RWS(pallet_rws::Call::claim { auction_id, .. }) => {
                        Some(auction_id) == allowed_auction.as_ref()
                    }
                    // For call operations on existing subscriptions, allow if no auction restriction
                    RuntimeCall::RWS(pallet_rws::Call::call { .. }) => {
                        allowed_auction.is_none()
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
            (ProxyType::RwsManager(None), ProxyType::RwsManager(_)) => true,
            (ProxyType::RwsManager(Some(a)), ProxyType::RwsManager(Some(b))) => a == b,
            _ => false,
        }
    }
}
```

### Complete User Story: IoT Subscription Management

This example demonstrates a real-world scenario where Alice owns a subscription and delegates device management to an IoT gateway.

```rust
// ═══════════════════════════════════════════════════════════════════
// SCENARIO: Alice owns a factory with IoT sensors. She wants the IoT
// gateway to execute transactions using her RWS subscription without
// giving it access to her main account keys.
// ═══════════════════════════════════════════════════════════════════

// STEP 1: Alice participates in auction and wins
// ───────────────────────────────────────────────
Timestamp::set_timestamp(1_000_000);

// Root starts auction for 30-day subscription
RWS::start_auction(
    RuntimeOrigin::root(),
    SubscriptionMode::Daily { days: 30 }
)?;
// → Auction 0 created

// Alice bids 100 XRT
RWS::bid(
    RuntimeOrigin::signed(ALICE),
    0,  // auction_id
    100 * XRT
)?;
// → Alice is winning bidder
// → 100 XRT reserved from Alice's balance

// Wait for auction to end
Timestamp::set_timestamp(1_000_000 + AuctionDuration::get() + 1);

// Alice claims the subscription
RWS::claim(
    RuntimeOrigin::signed(ALICE),
    0,     // auction_id
    None   // Alice will be the beneficiary
)?;
// → Subscription 0 created for Alice
// → 100 XRT burned
// → Alice can now use subscription for free transactions


// STEP 2: Alice adds IoT gateway as proxy
// ────────────────────────────────────────
const IOT_GATEWAY: AccountId = 0x123...; // Gateway account

// Alice creates a proxy for the IoT gateway with RwsManager type
Proxy::add_proxy(
    RuntimeOrigin::signed(ALICE),
    IOT_GATEWAY,
    ProxyType::RwsManager(None),  // Access to ALL RWS operations
    0  // No announcement delay
)?;
// → Gateway can now manage Alice's RWS subscriptions
// → Gateway CANNOT access Alice's balance, governance, etc.
// → Deposit reserved from Alice for proxy storage


// STEP 3: IoT gateway uses subscription on Alice's behalf
// ────────────────────────────────────────────────────────
Timestamp::set_timestamp(2_000_000);

// Gateway wants to record sensor data using Alice's subscription
let sensor_data = RuntimeCall::Datalog(
    pallet_datalog::Call::record {
        record: b"temperature:23.5C".to_vec().try_into().unwrap()
    }
);

// Gateway executes via proxy
Proxy::proxy(
    RuntimeOrigin::signed(IOT_GATEWAY),
    ALICE,  // Real account (subscription owner)
    Some(ProxyType::RwsManager(None)),
    Box::new(RuntimeCall::RWS(
        pallet_rws::Call::call {
            subscription_id: 0,
            call: Box::new(sensor_data)
        }
    ))
)?;
// → Gateway successfully uses Alice's subscription
// → Transaction executes with Pays::No (no fees)
// → Alice's subscription weight is deducted
// → Sensor data recorded to chain


// STEP 4: Alice monitors and controls access
// ───────────────────────────────────────────
// Alice can check her proxies at any time
let proxies = Proxy::proxies(ALICE);
// → Returns: [(IOT_GATEWAY, ProxyType::RwsManager(None), 0)]

// If gateway is compromised or decommissioned, Alice revokes access
Proxy::remove_proxy(
    RuntimeOrigin::signed(ALICE),
    IOT_GATEWAY,
    ProxyType::RwsManager(None),
    0
)?;
// → Gateway can no longer use Alice's subscription
// → Alice's deposit returned
// → Subscription remains active for Alice


// STEP 5: Alice can still use subscription directly
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

### Additional Usage Examples

#### Example 1: Time-Delayed Proxy for Security

Protect high-value subscriptions with announcement delays:

```rust
// Alice creates a proxy with 1-day announcement delay
Proxy::add_proxy(
    RuntimeOrigin::signed(ALICE),
    BOB,
    ProxyType::RwsManager(None),
    24 * HOURS  // Bob must announce 24 hours before acting
)?;

// Bob wants to use subscription
Proxy::announce(
    RuntimeOrigin::signed(BOB),
    ALICE,
    BlakeTwo256::hash_of(&rws_call)
)?;
// → Bob's intent is recorded on-chain
// → Alice has 24 hours to review and potentially revoke proxy

// After 24 hours, Bob can execute
Proxy::proxy_announced(
    RuntimeOrigin::signed(BOB),
    ALICE,
    ALICE,  // No other proxies involved
    Some(ProxyType::RwsManager(None)),
    Box::new(rws_call)
)?;
// → Transaction executes using Alice's subscription
```

#### Example 2: Multi-Signature Workflow for Team Subscriptions

Combine with multisig for team-managed subscriptions:

```rust
// Team creates a multisig account
let team_account = MultiAddress::Id(TEAM_MULTISIG);

// Team multisig wins auction and claims subscription
// (via standard multisig approval process)
RWS::bid(RuntimeOrigin::signed(TEAM_MULTISIG), 0, 200 * XRT)?;
// ... auction ends ...
RWS::claim(RuntimeOrigin::signed(TEAM_MULTISIG), 0, None)?;

// Team adds individual members as proxies
Proxy::add_proxy(
    RuntimeOrigin::signed(TEAM_MULTISIG),
    ALICE,
    ProxyType::RwsManager(None),
    0
)?;

Proxy::add_proxy(
    RuntimeOrigin::signed(TEAM_MULTISIG),
    BOB,
    ProxyType::RwsManager(None),
    0
)?;

// Now Alice or Bob can use the team subscription independently
Proxy::proxy(
    RuntimeOrigin::signed(ALICE),
    TEAM_MULTISIG,
    Some(ProxyType::RwsManager(None)),
    Box::new(RuntimeCall::RWS(pallet_rws::Call::call { ... }))
)?;
```

#### Example 3: Auction-Specific Proxy Restriction

Grant proxy access to a specific auction only:

```rust
// Root starts two auctions
RWS::start_auction(RuntimeOrigin::root(), SubscriptionMode::Daily { days: 7 })?;  // Auction 0
RWS::start_auction(RuntimeOrigin::root(), SubscriptionMode::Lifetime { tps: 50_000 })?;  // Auction 1

// Alice grants Bob permission to bid ONLY on auction 1
Proxy::add_proxy(
    RuntimeOrigin::signed(ALICE),
    BOB,
    ProxyType::RwsManager(Some(1)),  // Restricted to auction 1
    0
)?;

// Bob can bid on auction 1 using Alice's funds
Proxy::proxy(
    RuntimeOrigin::signed(BOB),
    ALICE,
    Some(ProxyType::RwsManager(Some(1))),
    Box::new(RuntimeCall::RWS(pallet_rws::Call::bid {
        auction_id: 1,
        amount: 150 * XRT
    }))
)?;
// → Success: Bid placed on behalf of Alice for auction 1

// But Bob CANNOT bid on auction 0
Proxy::proxy(
    RuntimeOrigin::signed(BOB),
    ALICE,
    Some(ProxyType::RwsManager(Some(1))),
    Box::new(RuntimeCall::RWS(pallet_rws::Call::bid {
        auction_id: 0,
        amount: 150 * XRT
    }))
)?;
// → Error: ProxyType filter rejects call (wrong auction)
```

#### Example 4: Automated Bot with Restricted RWS Access

Deploy autonomous bots with limited permissions:

```rust
// Alice creates a subscription management bot
const AUTO_BIDDER_BOT: AccountId = 0x456...;

// Grant bot RwsManager-only access (no balance transfers, governance, etc.)
Proxy::add_proxy(
    RuntimeOrigin::signed(ALICE),
    AUTO_BIDDER_BOT,
    ProxyType::RwsManager(None),
    0
)?;

// Bot can autonomously bid on auctions for Alice
Proxy::proxy(
    RuntimeOrigin::signed(AUTO_BIDDER_BOT),
    ALICE,
    Some(ProxyType::RwsManager(None)),
    Box::new(RuntimeCall::RWS(pallet_rws::Call::bid {
        auction_id: 5,
        amount: calculate_optimal_bid()
    }))
)?;
// → Bot successfully bids using Alice's account

// Bot CANNOT transfer Alice's funds
Proxy::proxy(
    RuntimeOrigin::signed(AUTO_BIDDER_BOT),
    ALICE,
    Some(ProxyType::RwsManager(None)),
    Box::new(RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
        dest: BOB,
        value: 100 * XRT
    }))
)?;
// → Error: ProxyType::RwsManager filter blocks non-RWS calls
```

### Security Considerations

#### Type Safety
- **Restricted Call Space**: `ProxyType::RwsManager` only allows RWS pallet calls
- **No Escalation**: Proxies cannot call `Proxy::add_proxy` or other privilege-granting functions
- **Compile-Time Guarantees**: Substrate's type system enforces restrictions

#### Auction-Level Granularity
- **Fine-Grained Control**: `RwsManager(Some(auction_id))` limits access to specific auctions
- **Prevents Overreach**: Bot restricted to auction 1 cannot affect auction 0
- **Flexible Permissions**: Combine auction restrictions with time delays for maximum control

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
- **Horizontal Access**: `RwsManager` proxies cannot escalate to `Any` or other types
- **Isolated Operations**: Actions limited to explicitly granted capabilities

### Use Cases

#### 1. IoT Device Fleet Management
- **Scenario**: Company with 1,000 sensors needing free transactions
- **Solution**: Company account owns subscription; each sensor is a proxy
- **Benefits**: 
  - Sensors operate autonomously without exposing company keys
  - Individual sensor compromise doesn't affect others (revoke single proxy)
  - Centralized subscription management with distributed execution

#### 2. Multi-Signature Team Subscriptions
- **Scenario**: DAO wants shared subscription for member activities
- **Solution**: Multisig account owns subscription; members are proxies
- **Benefits**:
  - Members can use subscription without multisig approval per transaction
  - Critical operations (new subscription, revoke member) require multisig
  - Clear audit trail of which member performed each action

#### 3. Automated Subscription Renewals
- **Scenario**: User wants bot to automatically bid on new auctions
- **Solution**: User grants bot `RwsManager` proxy with no auction restriction
- **Benefits**:
  - Bot monitors auctions and bids optimally
  - User retains control (can revoke bot anytime)
  - Bot cannot access funds or perform non-RWS operations

#### 4. Temporary Access for Maintenance/Audits
- **Scenario**: User needs contractor to debug subscription issues
- **Solution**: Grant contractor time-delayed `RwsManager` proxy for 7 days
- **Benefits**:
  - Contractor can test subscription usage
  - Time delay provides security window for owner to review
  - Automatic expiration (or manual revocation) after engagement

#### 5. Hierarchical Subscription Management
- **Scenario**: Organization with departments needing separate subscription access
- **Solution**: Parent account owns multiple subscriptions; each department is proxy for their subscription
- **Benefits**:
  - Department A's proxy can't access Department B's subscription
  - Central billing with departmental autonomy
  - Easy reorganization (revoke/add proxies as structure changes)

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