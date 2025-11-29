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