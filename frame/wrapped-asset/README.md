# Pallet Wrapped Asset

Bidirectional conversion between parachain native token and foreign asset representation on Asset Hub via XCM.

## Overview

The Wrapped Asset pallet enables users to convert their parachain's native token to and from a foreign asset representation on Asset Hub using XCM (Cross-Consensus Messaging).

### Key Features

- **Wrap**: Burn native tokens locally and send equivalent foreign assets from the sovereign account on Asset Hub
- **Unwrap**: Receive foreign assets via XCM and mint equivalent native tokens locally
- **Balance Tracking**: Maintains `TotalWrapped` to track the sovereign account's foreign asset balance
- **Safety**: Prevents over-wrapping by ensuring sufficient foreign assets exist in the sovereign account

## Use Cases

1. **Cross-chain Trading**: Users can wrap native tokens to trade them as foreign assets on Asset Hub DEXs
2. **Liquidity Provision**: Wrapped assets can be used to provide liquidity on Asset Hub
3. **Bridging**: Enables seamless movement of value between parachain and Asset Hub

## Configuration

Add the pallet to your runtime's `Cargo.toml`:

```toml
[dependencies]
pallet-wrapped-asset = { path = "../frame/wrapped-asset", default-features = false }
```

Configure the pallet in your runtime:

```rust
parameter_types! {
    // Location of your parachain's native token as foreign asset on Asset Hub
    pub ForeignAssetLocation: Location = Location::new(
        1,
        [Parachain(YOUR_PARA_ID), GeneralIndex(ASSET_ID)]
    );
    
    // Asset Hub location (typically Parachain 1000)
    pub AssetHubLocation: Location = Location::new(1, [Parachain(1000)]);
    
    // Fee amount in relay chain tokens for XCM execution on Asset Hub
    pub const XcmFeeAmount: u128 = 1_000_000_000; // 0.001 relay tokens
}

impl pallet_wrapped_asset::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type NativeCurrency = Balances;
    type ForeignAssetLocation = ForeignAssetLocation;
    type AssetHubLocation = AssetHubLocation;
    type XcmFeeAmount = XcmFeeAmount;
}
```

## XCM Integration

To enable unwrapping, integrate the pallet into your runtime's XCM configuration:

```rust
pub struct CustomAssetTransactor;

impl TransactAsset for CustomAssetTransactor {
    fn deposit_asset(
        what: &Asset,
        who: &Location,
        _context: &XcmContext,
    ) -> Result<(), XcmError> {
        match what {
            Asset {
                id: AssetId(location),
                fun: Fungible(amount),
            } if location == &ForeignAssetLocation::get() => {
                // Convert Location to AccountId
                let beneficiary = LocationToAccountId::convert(who.clone())
                    .ok_or(XcmError::InvalidLocation)?;
                
                // Call unwrap handler
                pallet_wrapped_asset::Pallet::<Runtime>::handle_incoming_unwrap(
                    beneficiary,
                    *amount,
                ).map_err(|_| XcmError::FailedToTransactAsset("Unwrap failed"))?;
                
                Ok(())
            },
            _ => DefaultAssetTransactor::deposit_asset(what, who, _context),
        }
    }
    
    fn withdraw_asset(
        what: &Asset,
        who: &Location,
        _context: Option<&XcmContext>,
    ) -> Result<xcm_executor::AssetsInHolding, XcmError> {
        // Handle withdrawals with your existing transactor
        DefaultAssetTransactor::withdraw_asset(what, who, _context)
    }
}
```

## Usage

### Wrapping Tokens

Users can wrap their native tokens using the `wrap_and_send` extrinsic:

```rust
// Wrap 100 tokens and send to yourself (default beneficiary)
WrappedAsset::wrap_and_send(origin, 100, None)?;

// Wrap 100 tokens and send to custom beneficiary
let beneficiary = Location::new(
    1,
    [AccountId32 { network: None, id: destination_account }]
);
WrappedAsset::wrap_and_send(origin, 100, Some(beneficiary))?;
```

**What happens:**
1. 100 native tokens are burned from the caller's account
2. `TotalWrapped` is decremented by 100
3. XCM message is sent to Asset Hub to:
   - Withdraw 100 foreign assets from sovereign account
   - Withdraw relay tokens for fees
   - Send assets to beneficiary

### Unwrapping Tokens

Unwrapping happens automatically when foreign assets arrive via XCM. Users need to:

1. On Asset Hub, send the foreign asset to your parachain's sovereign account with proper XCM instructions
2. The runtime's `AssetTransactor` will intercept the deposit and call `handle_incoming_unwrap`
3. Native tokens are minted to the beneficiary
4. `TotalWrapped` is incremented

## Query Functions

The pallet provides several query functions:

```rust
// Get total wrapped balance (sovereign account balance on Asset Hub)
let total = WrappedAsset::get_total_wrapped();

// Check if amount can be wrapped
let can_wrap = WrappedAsset::can_wrap(100);

// Get maximum wrappable amount
let max = WrappedAsset::max_wrappable();
```

## Setup Requirements

### 1. Sovereign Account Funding

The parachain's sovereign account on Asset Hub **must** be funded with relay chain tokens to pay for XCM execution fees.

**Calculate Sovereign Account:**
```rust
use sp_runtime::traits::AccountIdConversion;

let para_id = 2048; // Your parachain ID
let sovereign_account = para_id.into_account_truncating();
```

**Funding Amount:**
- Minimum: Depends on XCM execution costs and expected wrapping frequency
- Recommended: 10-100 relay tokens for regular usage

### 2. Foreign Asset Setup

Your native token must be registered as a foreign asset on Asset Hub:
1. Submit governance proposal on Asset Hub to register foreign asset
2. Note the asset ID assigned
3. Use this in `ForeignAssetLocation` configuration

### 3. Initial TotalWrapped

When deploying, `TotalWrapped` starts at 0. To bootstrap:
1. Manually deposit foreign assets into the sovereign account on Asset Hub
2. Call `handle_incoming_unwrap` to mint native tokens and increment `TotalWrapped`
3. OR send foreign assets from Asset Hub to users via XCM (automatic unwrapping)

## Security Considerations

### Conservation Invariant

**Critical:** `TotalWrapped` must always equal the foreign asset balance in the sovereign account on Asset Hub.

- ✅ Safe: Using `wrap_and_send` and `handle_incoming_unwrap` maintains this invariant
- ❌ Unsafe: Manually sending foreign assets from sovereign account without calling `wrap_and_send`
- ❌ Unsafe: Calling `handle_incoming_unwrap` without actual foreign asset deposits

### Origin Validation

Only Asset Hub should be able to trigger `handle_incoming_unwrap`. Validate the origin in your XCM configuration:

```rust
// In your AssetTransactor
fn deposit_asset(...) -> Result<(), XcmError> {
    // Validate the origin is from Asset Hub
    let origin_location = /* extract from context */;
    ensure!(
        origin_location == AssetHubLocation::get(),
        XcmError::InvalidLocation
    );
    
    // ... rest of implementation
}
```

### Fee Management

Monitor the sovereign account's relay token balance. If it runs too low:
- Wrapping operations will fail on Asset Hub
- Users won't be able to wrap tokens
- Solution: Top up the sovereign account with relay tokens

## Events

The pallet emits two events:

### NativeWrapped

Emitted when native tokens are wrapped and sent to Asset Hub.

```rust
NativeWrapped {
    who: AccountId,           // Account that wrapped tokens
    amount: Balance,          // Amount wrapped
    destination: Location,    // Where foreign assets were sent
}
```

### NativeUnwrapped

Emitted when foreign assets are received and unwrapped to native tokens.

```rust
NativeUnwrapped {
    who: AccountId,    // Account that received native tokens
    amount: Balance,   // Amount unwrapped
}
```

## Errors

| Error | Description | Resolution |
|-------|-------------|------------|
| `InsufficientBalance` | Caller doesn't have enough native tokens | User needs more native tokens |
| `InsufficientWrappedBalance` | Sovereign account doesn't have enough foreign assets | More tokens need to be unwrapped first |
| `InvalidAmount` | Amount is zero | Specify non-zero amount |
| `BurnFailed` | Failed to burn native tokens | Check Currency implementation |
| `MintFailed` | Failed to mint native tokens | Check Currency implementation |
| `XcmSendFailed` | XCM message failed to send | Check XCM configuration |
| `AmountOverflow` | Amount conversion overflow | Amount too large for Balance type |

## Common Pitfalls

### 1. Sovereign Account Not Funded

**Symptom:** Users can unwrap but cannot wrap.

**Solution:** Fund sovereign account on Asset Hub with relay tokens.

### 2. Wrong ForeignAssetLocation

**Symptom:** Unwrapping doesn't work.

**Solution:** Verify `ForeignAssetLocation` matches actual foreign asset location on Asset Hub.

### 3. TotalWrapped Out of Sync

**Symptom:** Can't wrap even though sovereign account has foreign assets.

**Solution:** This indicates manual foreign asset transfers. Avoid direct transfers; always use the pallet.

### 4. XCM Fee Too Low

**Symptom:** Wrapping transactions succeed but assets don't arrive.

**Solution:** Increase `XcmFeeAmount` parameter.

## Testing

Run the pallet tests:

```bash
cargo test -p pallet-wrapped-asset
```

Run tests with output:

```bash
cargo test -p pallet-wrapped-asset -- --nocapture
```

## License

Apache-2.0

## Support

For issues or questions:
- GitHub Issues: https://github.com/airalab/robonomics/issues
- Documentation: https://wiki.robonomics.network
