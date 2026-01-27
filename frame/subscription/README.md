# Subscription Pallet 2.0 - Technical Guide

## Overview

The Subscription Pallet provides a decentralized auction system for obtaining subscriptions that enable free transaction execution on the Robonomics Network. Users bid on subscriptions, and winning bidders receive the ability to execute transactions without paying fees, metered by computational weight and time.

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

## 🔌 Transaction Extension: Fee-less Transactions

### What is a Transaction Extension?

A **transaction extension** is a Substrate mechanism that allows customization of transaction validation and execution without modifying individual pallets. It wraps around transactions and can:
- Validate transactions before they execute
- Modify transaction behavior (like fee payment)
- Track transaction execution
- Work with any pallet's extrinsics

The Subscription Transaction Extension (`ChargeRwsTransaction`) enables **opt-in fee-less transactions** for subscription holders.

---

### Why Use Transaction Extension?

**Traditional Approach (Wrapper Extrinsic):**
```rust
// ❌ Old way: Wrap every call in Subscription::call(subscription_id, call)
Subscription::call(subscription_id: 0, call: Box::new(datalog::record(data)))
```
- Requires wrapping every call
- Limited to specific call types
- More complexity for users

**Transaction Extension Approach:**
```rust
// ✅ New way: Sign any transaction with RWS extension
// Extension is configured during signing with subscription_owner and subscription_id
// The extension is encoded in the SignedExtra tuple, not through a method chain
datalog::record(data)
    // When signing, ChargeRwsTransaction is set to:
    // Enabled { subscription_owner: alice, subscription_id: 0 }
```
- Works with ANY extrinsic
- Clean API
- Per-transaction opt-in
- Fully transparent

---

### How It Works

```
╔════════════════════════════════════════════════════════════════════╗
║              TRANSACTION EXTENSION FLOW                            ║
╚════════════════════════════════════════════════════════════════════╝

STEP 1: USER SIGNS TRANSACTION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
┌────────────────────────────────────┐
│ User creates transaction           │
│ • Any extrinsic (datalog, launch)  │
│ • Adds RWS extension parameter     │
└────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ ChargeRwsTransaction::Enabled      │
│ { owner: alice,                    │
│   subscription_id: 0 }             │
└────────────────────────────────────┘
         │
         │ Transaction submitted to network
         ▼


STEP 2: VALIDATION (validate)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
┌────────────────────────────────────┐
│ Extension checks:                  │
│ ✓ Subscription exists?             │
│ ✓ Subscription active?             │
│ ✓ Not expired?                     │
│ ✓ Sufficient free_weight?          │
└────────────────────────────────────┘
         │
         │ ✓ Valid → Continue
         │ ✗ Invalid → Reject (InvalidTransaction::Payment)
         ▼


STEP 3: PRE-DISPATCH (pre_dispatch)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
┌────────────────────────────────────┐
│ Final check before execution       │
│ • Re-validates subscription        │
│ • Records owner & subscription_id  │
└────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ Returns: RwsPreDispatch            │
│ • pays_no_fee: true                │
│ • subscription_owner: Alice        │
│ • subscription_id: 0               │
└────────────────────────────────────┘
         │
         ▼


STEP 4: TRANSACTION EXECUTES
━━━━━━━━━━━━━━━━━━━━━━━━━━━━
┌────────────────────────────────────┐
│ Extrinsic runs normally            │
│ • No fees charged (Pays::No)       │
│ • Standard execution               │
└────────────────────────────────────┘
         │
         ▼


STEP 5: POST-DISPATCH (post_dispatch)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
┌────────────────────────────────────┐
│ After execution:                   │
│ • Accumulate free_weight           │
│ • Deduct used weight               │
│ • Update last_update timestamp     │
│ • Emit SubscriptionUsed event      │
└────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ Transaction Complete ✓             │
└────────────────────────────────────┘
```

---

### Usage Examples

#### Example 1: JavaScript/TypeScript (Polkadot.js)

```typescript
import { ApiPromise, WsProvider } from '@polkadot/api';

// Connect to Robonomics
const api = await ApiPromise.create({
  provider: new WsProvider('wss://kusama.rpc.robonomics.network')
});

// Get account
const alice = /* ... keyring account ... */;

// ✅ OPTION A: Use subscription for fee-less transaction
// Create the transaction
const tx = api.tx.datalog.record('Temperature: 23.5°C');

// Sign with RWS extension enabled
const signedTx = await tx.signAsync(alice, { nonce: -1 });
// Note: RWS extension parameters (Enabled/Disabled with owner/subscription_id) 
// are encoded in the transaction extension data when signing

// ✅ OPTION B: Pay normal fees  
// When RWS extension is Disabled, normal fees apply
const tx = api.tx.datalog.record('Temperature: 23.5°C');
const signedTx = await tx.signAsync(alice, { nonce: -1 });

// ✅ OPTION C: Omit extension (default = Disabled)
await api.tx.datalog
  .record('Temperature: 23.5°C')
  .signAndSend(alice);  // Normal paid transaction
```

#### Example 2: Multiple Subscriptions

```typescript
// User can have multiple subscriptions (0, 1, 2, ...)
// Choose which one to use per transaction

// Use subscription 0 (Daily)
await api.tx.launch
  .launch(target, parameter)
  .signAndSend(alice, {
    rwsAuction: { Enabled: { subscriptionId: 0 } }
  });

// Use subscription 1 (Lifetime)
await api.tx.datalog
  .record(data)
  .signAndSend(alice, {
    rwsAuction: { Enabled: { subscriptionId: 1 } }
  });
```

#### Example 3: Rust (Node/Runtime)

```rust
use pallet_robonomics_subscription::ChargeRwsTransaction;

// Creating a transaction with RWS extension
let call = RuntimeCall::Datalog(
    pallet_robonomics_datalog::Call::record { 
        record: b"sensor_data".to_vec() 
    }
);

// Enable RWS subscription
let rws_extension = ChargeRwsTransaction::Enabled {
    subscription_id: 0,
};

// Build signed extra tuple
let extra = (
    // ... other extensions ...
    rws_extension,
    // ... ChargeTransactionPayment comes after ...
);

// Create and submit transaction
let xt = UncheckedExtrinsic::new_signed(
    call,
    account,
    signature,
    extra,
);
```

#### Example 4: Error Handling

```typescript
try {
  await api.tx.datalog.record(data).signAndSend(alice, {
    rwsAuction: { Enabled: { subscriptionId: 0 } }
  });
} catch (error) {
  // Common errors:
  // - InvalidTransaction::Payment: No subscription or expired
  // - InvalidTransaction::ExhaustsResources: Insufficient free_weight
  // - NoSubscription: Subscription doesn't exist
  
  if (error.message.includes('Payment')) {
    console.log('Subscription invalid or expired');
    // Fall back to normal paid transaction
    await api.tx.datalog.record(data).signAndSend(alice);
  }
}
```

---

### Key Differences: Old vs New

| Aspect | Old (call extrinsic) | New (Transaction Extension) |
|--------|---------------------|----------------------------|
| **Usage** | `Subscription::call(id, Box::new(call))` | Any extrinsic + RWS parameter |
| **Flexibility** | Limited to compatible calls | Works with ANY extrinsic |
| **API** | Wrapper required | Direct transaction signing |
| **Opt-in** | Per-call basis (implicit) | Per-transaction (explicit) |
| **Transparency** | Hidden in wrapper | Clear in signature |
| **Removal** | ❌ Removed in v4.0 | ✅ Recommended approach |

---


### Monitoring & Events

Track subscription usage via events:

```typescript
// Subscribe to subscription usage events
api.query.system.events((events) => {
  events.forEach((record) => {
    const { event } = record;
    
    if (api.events.rwsAuction.SubscriptionUsed.is(event)) {
      const [account, subId, weight] = event.data;
      console.log(`Subscription ${subId} used by ${account}`);
      console.log(`Weight consumed: ${weight}`);
    }
  });
});
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

## 🔐 Lifetime Subscriptions via Asset Locking

In addition to the auction-based subscription model, users can acquire **Lifetime subscriptions directly** by locking assets from pallet-assets. This provides an alternative path that doesn't require waiting for auctions or competing with other bidders.

### Overview and Benefits

The asset locking mechanism offers several advantages:

✅ **Immediate Activation** - No waiting for auctions or competing with other bidders  
✅ **Recoverable Cost** - Assets are locked, not burned - can be recovered via `stop_lifetime()`  
✅ **Flexible TPS** - Configure your transaction capacity by adjusting locked amount  
✅ **No Competition** - Direct acquisition without market-driven bidding  
✅ **On-Demand Control** - Start and stop subscriptions at will  

### Asset-to-TPS Conversion Formula

The relationship between locked assets and TPS allocation is governed by the `AssetToTpsRatio` configuration parameter, which uses Substrate's `Permill` type for precise ratio representation:

```
TPS (μTPS) = Locked Asset Amount × AssetToTpsRatio.deconstruct()
```

**Formula Components:**
- `AssetToTpsRatio`: A `Permill` value representing μTPS per token
- `Locked Asset Amount`: Number of tokens locked in pallet account
- `TPS (μTPS)`: Resulting micro-transactions per second (1 TPS = 1,000,000 μTPS)

**Example Calculations:**

With `AssetToTpsRatio = Permill::from_parts(100)` (100 μTPS per token):

| Locked Amount | Calculation | Result (μTPS) | Result (TPS) |
|---------------|-------------|---------------|--------------|
| 100 tokens    | 100 × 100   | 10,000 μTPS   | 0.01 TPS     |
| 500 tokens    | 500 × 100   | 50,000 μTPS   | 0.05 TPS     |
| 1,000 tokens  | 1000 × 100  | 100,000 μTPS  | 0.1 TPS      |
| 10,000 tokens | 10000 × 100 | 1,000,000 μTPS| 1.0 TPS      |

With `AssetToTpsRatio = Permill::from_parts(1000)` (1,000 μTPS per token):

| Locked Amount | Calculation | Result (μTPS) | Result (TPS) |
|---------------|-------------|---------------|--------------|
| 10 tokens     | 10 × 1000   | 10,000 μTPS   | 0.01 TPS     |
| 50 tokens     | 50 × 1000   | 50,000 μTPS   | 0.05 TPS     |
| 100 tokens    | 100 × 1000  | 100,000 μTPS  | 0.1 TPS      |
| 1,000 tokens  | 1000 × 1000 | 1,000,000 μTPS| 1.0 TPS      |

Using `Permill` provides better precision and prevents overflow issues in the calculation.

### Lifecycle Diagram

```
╔════════════════════════════════════════════════════════════════════╗
║           ASSET LOCKING SUBSCRIPTION LIFECYCLE                     ║
╚════════════════════════════════════════════════════════════════════╝

PHASE 1: LOCK ASSETS & ACTIVATE
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
┌────────────────────────────┐
│ User                       │
│ Has: 500 asset tokens      │
└────────┬───────────────────┘
         │ start_lifetime(500)
         │
         ▼
┌────────────────────────────────────┐
│ Asset Transfer                     │
│ • 500 tokens → Pallet Account      │
│ • Assets LOCKED (not burned)       │
└────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ TPS Calculation                    │
│ • Amount: 500                      │
│ • Ratio: 100 μTPS per token        │
│ • Result: 50,000 μTPS (0.05 TPS)   │
└────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ Subscription Created               │
│ • subscription_id: 0               │
│ • mode: Lifetime { tps: 50000 }    │
│ • free_weight: 0                   │
│ • Never expires                    │
└────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ Storage Updates                    │
│ • Subscription(User, 0) → Ledger   │
│ • LockedAssets(User, 0) → 500      │
└────────────────────────────────────┘
         │
         │ Emits: SubscriptionActivated(User, 0)
         ▼


PHASE 2: ACTIVE USAGE
━━━━━━━━━━━━━━━━━━━━
┌────────────────────────────────────┐
│ User executes free transactions    │
│ └─▶ call(subscription_id: 0, ...)  │
└────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ Weight Accumulation                │
│ • Time passes → free_weight grows  │
│ • Rate: 50,000 μTPS (0.05 TPS)     │
│ • Call deducts used weight         │
└────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ Transaction Executes               │
│ • Pays::No (feeless)               │
│ • Assets remain locked             │
└────────────────────────────────────┘
         │
         │ (User decides to stop)
         ▼


PHASE 3: STOP & UNLOCK
━━━━━━━━━━━━━━━━━━━━━
┌────────────────────────────────────┐
│ User                               │
│ └─▶ stop_lifetime(subscription_id: 0)│
└────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ Validation                         │
│ • Subscription exists?             │
│ • Has locked assets?               │
│ • Caller is owner?                 │
└────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ Asset Unlock                       │
│ • 500 tokens → User Account        │
│ • Pallet Account → User            │
└────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────┐
│ Storage Cleanup                    │
│ • Remove Subscription(User, 0)     │
│ • Remove LockedAssets(User, 0)     │
└────────────────────────────────────┘
         │
         │ Emits: SubscriptionStopped(User, 0)
         ▼
    [ASSETS RECOVERED]
```

### Complete Usage Examples

#### Example 1: Basic Lifetime Subscription

```rust
// Alice wants 0.05 TPS (50,000 μTPS) subscription
// With AssetToTpsRatio = Permill::from_parts(100) (100 μTPS per token)
// She needs: 50,000 / 100 = 500 tokens

// STEP 1: Alice locks assets to create subscription
Subscription::start_lifetime(
    RuntimeOrigin::signed(alice),
    500 // amount of assets to lock
)?;
// → Alice's subscription_id is 0 (her first subscription)
// → 500 tokens transferred to pallet account
// → Subscription with 50,000 μTPS (0.05 TPS) created
// → Emits: SubscriptionActivated(alice, 0)

// STEP 2: Alice uses subscription for feeless transactions
Subscription::call(
    RuntimeOrigin::signed(alice),
    0, // subscription_id
    Box::new(RuntimeCall::Datalog(
        pallet_datalog::Call::record {
            record: b"sensor_data:temp=23.5C".to_vec()
        }
    ))
)?;
// → Transaction executes with Pays::No (no fees)
// → free_weight deducted from subscription
// → Alice's 500 tokens remain locked

// STEP 3: Alice stops subscription to recover assets
Subscription::stop_lifetime(
    RuntimeOrigin::signed(alice),
    0 // subscription_id
)?;
// → 500 tokens returned to alice
// → Subscription removed from storage
// → Emits: SubscriptionStopped(alice, 0)
```

#### Example 2: Multiple Subscriptions with Different TPS

```rust
// Bob wants multiple subscriptions with different capacities

// Subscription 0: Low TPS for monitoring (0.01 TPS = 10,000 μTPS)
Subscription::start_lifetime(
    RuntimeOrigin::signed(bob),
    100 // 100 × 100 = 10,000 μTPS
)?;

// Subscription 1: High TPS for active operations (0.5 TPS = 500,000 μTPS)
Subscription::start_lifetime(
    RuntimeOrigin::signed(bob),
    5000 // 5000 × 100 = 500,000 μTPS
)?;

// Bob uses subscription 0 for monitoring
Subscription::call(
    RuntimeOrigin::signed(bob),
    0,
    Box::new(monitor_call)
)?;

// Bob uses subscription 1 for heavy operations
Subscription::call(
    RuntimeOrigin::signed(bob),
    1,
    Box::new(heavy_operation_call)
)?;

// Bob can stop specific subscriptions independently
Subscription::stop_lifetime(
    RuntimeOrigin::signed(bob),
    0 // Stop only subscription 0, subscription 1 remains active
)?;
```

#### Example 3: IoT Fleet Management

```rust
// Company deploys 10 IoT devices, each needs its own subscription
// Device accounts could be generated from a seed or derived from company account

// Example device account generation (simplified)
let device_accounts: Vec<AccountId> = (0..10)
    .map(|i| derive_account_from_seed(&format!("company-device-{}", i)))
    .collect();

for (device_id, device_account) in device_accounts.iter().enumerate() {
    // Each device gets 0.02 TPS (20,000 μTPS)
    // Required: 20,000 / 100 = 200 tokens per device
    Subscription::start_lifetime(
        RuntimeOrigin::signed(device_account.clone()),
        200
    )?;
    // → Each device gets subscription_id 0 (their first subscription)
    // → Total locked: 10 × 200 = 2,000 tokens
}

// Each device operates independently
for device_account in &device_accounts {
    Subscription::call(
        RuntimeOrigin::signed(device_account.clone()),
        0, // Each device's first subscription
        Box::new(device_operation.clone())
    )?;
}

// Decommission device 5 (index 5 in the array)
Subscription::stop_lifetime(
    RuntimeOrigin::signed(device_accounts[5].clone()),
    0
)?;
// → 200 tokens recovered from device 5
// → Other devices unaffected
```

### Comparison: Asset Locking vs Auction-Based

| Feature | Asset Locking | Auction-Based |
|---------|--------------|---------------|
| **Acquisition Speed** | ⚡ Immediate | ⏳ Wait for auction + bidding period |
| **Cost Type** | 🔒 Locked (recoverable) | 🔥 Burned (permanent) |
| **Cost Recovery** | ✅ Yes, via `stop_lifetime()` | ❌ No recovery |
| **Duration** | ♾️ Unlimited (until stopped) | ♾️ Lifetime or ⏰ Daily (fixed) |
| **TPS Flexibility** | 🎛️ Configurable via amount | 🎯 Fixed by auction type |
| **Competition** | ✅ None required | ⚔️ Must outbid others |
| **Entry Barrier** | 💰 Need lockable assets | 💰 Need burnable tokens |
| **Exit Flexibility** | ✅ Stop anytime | ❌ Cannot stop/refund |
| **Best For** | Dynamic needs, temporary use | Long-term commitment |
| **Risk** | 🛡️ Low (recoverable) | ⚠️ High (permanent cost) |

### Storage Structure: LockedAssets

The `LockedAssets` storage is a double map that tracks asset-locked subscriptions:

```
LockedAssets: StorageDoubleMap<AccountId, u32, AssetBalance>
                                  ↑        ↑       ↑
                                  │        │       └─ Amount of assets locked
                                  │        └─────────── Subscription ID
                                  └──────────────────── Account owner
```

**Purpose:**
- Enables `stop_lifetime()` to determine locked amount for unlock
- Distinguishes asset-locked subscriptions from auction-based ones
- Supports multiple subscriptions per account with different locked amounts

**Example Storage State:**

```
LockedAssets(Alice, 0) = 500
LockedAssets(Alice, 1) = 1000
LockedAssets(Bob, 0) = 200
LockedAssets(Bob, 1) = None  ← Bob's subscription 1 is auction-based
```

**Lifecycle:**
- **Created**: When `start_lifetime()` is called
- **Queried**: When `stop_lifetime()` validates unlock eligibility
- **Removed**: When `stop_lifetime()` completes successfully

**Relationship to Subscription Storage:**

```
┌─────────────────────────┐         ┌──────────────────────────┐
│ Subscription(Alice, 0)  │         │ LockedAssets(Alice, 0)   │
│ ─────────────────────── │         │ ──────────────────────── │
│ free_weight: 1_000_000  │ ◄─────► │ amount: 500              │
│ mode: Lifetime{50000}   │   1:1   │                          │
│ issue_time: T₀          │  Link   │ For asset-locked subs    │
│ last_update: T₁         │         │ only                     │
└─────────────────────────┘         └──────────────────────────┘

┌─────────────────────────┐         ┌──────────────────────────┐
│ Subscription(Bob, 0)    │         │ LockedAssets(Bob, 0)     │
│ ─────────────────────── │         │ ──────────────────────── │
│ mode: Daily{30}         │    ✖    │ None                     │
│ (auction-based)         │  No     │                          │
│                         │  Link   │ Auction subs don't have  │
└─────────────────────────┘         │ locked assets            │
                                     └──────────────────────────┘
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


LockedAssets: DoubleMap<AccountId, u32, AssetBalance>
├─ LockedAssets(Alice, 1) ─┐
│                          ▼
│                        500  ← Asset-locked: 500 tokens locked
│
└─ LockedAssets(Bob, 0) ───┐
                           ▼
                         200  ← Asset-locked: 200 tokens locked

Note: LockedAssets entries only exist for subscriptions created
      via start_lifetime(). Auction-based subscriptions (like
      Alice's subscription 0) do not have entries in this storage.
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

---

#### `start_lifetime(amount)`
Start a lifetime subscription by locking assets.

**Requirements:**
- Caller must have sufficient balance of the configured lifetime asset
- Amount must be convertible to valid TPS value

**Parameters:**
- `amount: AssetBalance` - Amount of assets to lock

**Effects:**
- Transfers assets from caller to pallet account (locks them)
- Calculates TPS using formula: `amount × AssetToTpsRatio`
- Creates new Lifetime subscription for caller
- Assigns subscription_id (incremental per account)
- Stores locked amount in `LockedAssets` storage
- Emits `SubscriptionActivated` event

**Example:**
```rust
// Lock 500 asset tokens
// With AssetToTpsRatio = Permill::from_parts(100) (100 μTPS per token)
// This gives: 500 × 100 = 50,000 μTPS (0.05 TPS)
start_lifetime(500)
```

---

#### `stop_lifetime(subscription_id)`
Stop a lifetime subscription and unlock assets.

**Requirements:**
- Caller must own the subscription
- Subscription must be a Lifetime subscription created via asset locking
- Subscription must exist in `LockedAssets` storage

**Parameters:**
- `subscription_id: u32` - The subscription ID to stop

**Effects:**
- Transfers locked assets from pallet account back to caller
- Removes subscription from storage
- Removes locked amount record from `LockedAssets` storage
- Emits `SubscriptionStopped` event

**Example:**
```rust
// Stop subscription 0 and recover locked assets
stop_lifetime(0)
```

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
| `Call` | Runtime call type | Type for dispatchable calls that can be executed via subscriptions |
| `Time` | Time provider | Source for current time/timestamp |
| `Moment` | Timestamp type | Time representation for durations and timestamps (typically milliseconds) |
| `AuctionCurrency` | Currency trait | The currency used for auction bids (typically native token) |
| `Assets` | Fungibles trait | Interaction with pallet-assets for asset locking functionality |
| `PalletId` | PalletId | Unique identifier for the pallet account that holds locked assets |
| `LifetimeAssetId` | Asset ID | Specific asset ID used for lifetime subscription asset locking |
| `AssetToTpsRatio` | Permill | Conversion ratio: μTPS per 1 locked asset token (e.g., Permill::from_parts(100) = 100 μTPS per token) |
| `RuntimeEvent` | Event type | The overarching event type for runtime |
| `ReferenceCallWeight` | `u64` | Weight unit for TPS calculations (standard transaction weight) |
| `AuctionDuration` | `Moment` | Duration of bidding period after first bid (in milliseconds) |
| `MinimalBid` | `Balance` | Minimum amount for first bid in an auction |
| `StartAuctionOrigin` | Origin type | Origin authorized to start auctions (typically root or governance) |
| `WeightInfo` | Weight info trait | Benchmarked weights for extrinsic operations |

---

## 📡 Events

| Event | Parameters | Description |
|-------|------------|-------------|
| `AuctionStarted` | `(u32)` | New auction created |
| `NewBid` | `(u32, AccountId, Balance)` | Bid placed on auction |
| `AuctionFinished` | `(u32)` | Auction claimed and subscription created |
| `SubscriptionActivated` | `(AccountId, u32)` | Subscription activated for user (from auction or asset locking) |
| `SubscriptionStopped` | `(AccountId, u32)` | Asset-locked subscription stopped and assets unlocked |
| `RwsCall` | `(AccountId, u32, DispatchResult)` | Free transaction executed via subscription |

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

The Subscription pallet integrates with `pallet-proxy` to enable **subscription sharing**. This allows subscription owners to delegate usage of their subscriptions to other accounts without transferring ownership.

### Overview

The `ProxyType::RwsUser(subscription_id)` variant allows controlled delegation:
- **Single Purpose**: Only allows using a specific subscription via `Subscription::call`
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
    /// RWS subscription user - allows using a specific subscription via Subscription::call
    /// The parameter is the subscription_id that the proxy can use
    RwsUser(u32),
}

impl frame_support::traits::InstanceFilter<RuntimeCall> for ProxyType {
    fn filter(&self, c: &RuntimeCall) -> bool {
        match self {
            ProxyType::Any => true,
            ProxyType::RwsUser(allowed_subscription_id) => {
                // Only allow Subscription::call operations for the specific subscription
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
Subscription::start_auction(
    RuntimeOrigin::root(),
    SubscriptionMode::Lifetime { tps: 1_000_000 }  // 1 TPS
)?;

// Alice bids and wins
Subscription::bid(RuntimeOrigin::signed(ALICE), 0, 100 * XRT)?;
// → Alice is winning bidder
// → 100 XRT reserved from Alice's balance

// Wait for auction to end
Timestamp::set_timestamp(1_000_000 + AuctionDuration::get() + 1);

// Alice claims the subscription
Subscription::claim(RuntimeOrigin::signed(ALICE), 0, None)?;
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
Subscription::call(
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
Subscription::bid(RuntimeOrigin::signed(TEAM_MULTISIG), 0, 200 * XRT)?;
// ... auction ends ...
Subscription::claim(RuntimeOrigin::signed(TEAM_MULTISIG), 0, None)?;

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
- **Restricted Call Space**: `ProxyType::RwsUser` only allows `Subscription::call` for a specific subscription
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