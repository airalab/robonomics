# Robonomics XCM Teleport Pallet (`TeleportXrt`)

A specialized pallet for sending native XRT tokens from the Robonomics parachain to Asset Hub using XCM teleport.

## Overview

This pallet provides a simplified and restricted XCM teleport interface specifically designed for the Robonomics network. It enables users to send native XRT tokens to the Asset Hub parachain for cross-chain operations with configurable execution fees.

## API

### Extrinsic: `send`

```rust
pub fn send(
    origin: OriginFor<T>,
    beneficiary: Location,
    amount: u128,
) -> DispatchResultWithPostInfo
```

**Parameters:**
- `origin`: Signed origin (sender account)
- `beneficiary`: XCM Location of recipient (typically AccountId32 on Asset Hub)
- `amount`: Amount of token to send (as u128)

**Errors:**
- `BurnFailure`: Failed to burn assets locally
- `SendFailure`: XCM message send failed
- `CannotReanchor`: Failed to reanchor asset for destination chain (usually configuration issues)

## Usage Example

```rust
use pallet_robonomics_teleport;
use xcm::prelude::*;

// Send 1000 XRT to beneficiary on Asset Hub
let beneficiary_id = [0x01; 32]; // AccountId32 on Asset Hub
let beneficiary = Location::new(
    0,
    [AccountId32 {
        network: None,
        id: beneficiary_id,
    }],
);
let amount = 1_000_000_000_000; // 1000 XRT (12 decimals)

TeleportXrt::send(
    RuntimeOrigin::signed(alice),
    beneficiary,
    amount,
)?;
```

## Configuration

Runtime configuration for the pallet:

```rust
use frame_support::parameter_types;
use xcm::prelude::*;

parameter_types! {
    // Asset Hub location (typically parachain 1000)
    pub AssetHubLocation: Location = Location::new(1, [Parachain(1000)]);
    
    // Fee asset (relay chain native asset with amount)
    // Note: Parachain account on destination chain will cover execution fees
    pub TeleportFeeAsset: Asset = Asset {
        id: AssetId(Location::parent()),
        fun: Fungibility::Fungible(10_000_000_000), // 1 DOT (10 decimals)
    };
    
    // Parachain location for fee refunds
    pub ParachainLocation: Location = Location::new(1, [Parachain(2000)]);
    
    // Universal location for asset reanchoring
    pub UniversalLocation: InteriorLocation = [
        GlobalConsensus(NetworkId::Kusama),
        Parachain(2000)
    ].into();
    
    // Native asset ID
    pub NativeAssetId: AssetId = AssetId(Location::here());
    
    // Max weight for local XCM execution
    pub TeleportMaxWeight: Weight = Weight::from_parts(10_000_000, 10_000);
}

impl pallet_robonomics_teleport::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmPallet = PolkadotXcm;
    type WeightInfo = weights::pallet_robonomics_teleport::WeightInfo<Runtime>;
    type MaxWeight = TeleportMaxWeight;
    type AssetId = NativeAssetId;
    type FeeAsset = TeleportFeeAsset;
    type TargetLocation = AssetHubLocation;
    type ParachainLocation = ParachainLocation;
    type UniversalLocation = UniversalLocation;
}
```

**Configuration Parameters:**
- `RuntimeEvent`: Runtime event type for event emission
- `XcmPallet`: XCM pallet for execute and send operations
- `WeightInfo`: Weight information from benchmarks
- `MaxWeight`: Maximum weight for local XCM execution
- `AssetId`: Asset identifier (For native asset use Location::here())
- `FeeAsset`: Execution fee asset and amount (Relay token for Asset Hub)
- `TargetLocation`: Destination location (Usually Asset Hub)
- `ParachainLocation`: This parachain's location (for execution fee refunds)
- `UniversalLocation`: Universal location for asset reanchoring

## How It Works

### XCM Message Flow

The `send` extrinsic constructs and sends two XCM messages:

**Local Execution (burn assets):**
1. **WithdrawAsset** - Withdraws native assets from sender
2. **ExpectAsset** - Validates assets in holding
3. **BurnAsset** - Burns the assets (preparing for teleport)

**Remote Message (to Asset Hub):**
1. **WithdrawAsset** - Withdraws fee asset from parachain account
2. **PayFees** - Pays execution fees using withdrawn relay asset
3. **ReceiveTeleportedAsset** - Receives the teleported assets
4. **DepositAsset** - Deposits received assets to beneficiary
5. **RefundSurplus** - Refunds unused execution fees
6. **DepositAsset** - Returns refunded fees to parachain account
### Teleport Semantics

Teleport is a trust-based transfer mechanism where:
- Assets are **burned** on the source chain (Robonomics)
- Assets are **minted** on the destination chain (Asset Hub)
- Both chains must trust each other's asset issuance
- No additional proofs or confirmations required

### Fee Payment

Fees on Asset Hub are paid from the **parachain's account** using relay chain assets:
- Configured via `FeeAsset` parameter at runtime level
- Fee amount is pre-determined in configuration
- Surplus fees are refunded back to the parachain account
- Ensure parachain has sufficient relay tokens on Asset Hub

## Error Handling

The pallet implements comprehensive error handling:

| Error | Description | Cause |
|-------|-------------|-------|
| `BurnFailure` | Failed to burn assets locally | Insufficient balance or XCM execution error |
| `CannotReanchor` | Failed to reanchor asset | Invalid configuration or unsupported asset |
| `SendFailure` | XCM message send failed | XCM router error or destination unreachable |

## Security Considerations

- ✅ **Single Destination**: Hardcoded to Asset Hub prevents misuse
- ✅ **Native Asset Only**: No foreign asset support reduces complexity
- ✅ **Input Validation**: All parameters validated before execution
- ✅ **Safe Conversions**: Amount overflow handled gracefully
- ✅ **No Panics**: All unwraps replaced with proper error handling
- ✅ **Trust Model**: Relies on teleport trust relationship with Asset Hub

## Limitations

- **Fixed Destination**: Cannot send to other parachains
- **Native Asset Only**: Cannot send foreign or multi-assets
- **Trust Required**: Both chains must support teleport
- **No Retry Logic**: Failed transfers must be initiated again

## License

Apache-2.0
