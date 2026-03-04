# Robonomics XCM Teleport Pallet (`TeleportXrt`)

A specialized pallet for sending native XRT tokens from the Robonomics parachain to Asset Hub using XCM teleport with explicit fee control.

## Overview

This pallet provides a simplified and restricted XCM teleport interface specifically designed for the Robonomics network. It enables users to send native XRT tokens to the Asset Hub parachain for cross-chain operations with precise control over execution fees.

## Features

- **Single Asset Support**: Only supports native asset (XRT via pallet_balances)
- **Hardcoded Destination**: Asset Hub parachain only (para ID 1000)
- **Explicit Fee Control**: Separate fee parameter for relay chain asset fees
- **Modern XCM v5**: Uses InitiateTransfer and PayFees instructions
- **Simple API**: Beneficiary as raw AccountId32 bytes ([u8; 32])
- **Automatic Execution**: InitiateTransfer handles local execution

## API

### Extrinsic: `send`

```rust
pub fn send(
    origin: OriginFor<T>,
    beneficiary: [u8; 32],
    amount: BalanceOf<T>,
    fee: u128,
) -> DispatchResult
```

**Parameters:**
- `origin`: Signed origin (sender account)
- `beneficiary`: 32-byte AccountId32 of recipient on Asset Hub
- `amount`: Amount of native XRT to send
- `fee`: Relay chain asset amount for execution fees on Asset Hub

**Errors:**
- `AmountOverflow`: Amount exceeds u128::MAX
- `InvalidAssetFilter`: Asset transfer filter construction failed
- `SendFailure`: XCM message send failed

## Usage Example

```rust
use pallet_robonomics_teleport;

// Send 1000 XRT to beneficiary on Asset Hub
let beneficiary = [0x01; 32]; // AccountId32 on Asset Hub
let amount = 1_000_000_000_000; // 1000 XRT (12 decimals)
let fee = 50_000_000; // 0.05 DOT for fees (10 decimals)

TeleportXrt::send(
    RuntimeOrigin::signed(alice),
    beneficiary,
    amount,
    fee
)?;
```

## Configuration

Runtime configuration for the pallet:

```rust
use frame_support::parameter_types;
use xcm::prelude::*;

parameter_types! {
    // Asset Hub is typically parachain 1000 on the relay chain
    pub AssetHubLocation: Location = Location::new(1, [Parachain(1000)]);
}

impl pallet_robonomics_teleport::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type XcmSender = XcmRouter;
    type AssetHubLocation = AssetHubLocation;
}
```

**Configuration Parameters:**
- `RuntimeEvent`: Runtime event type for event emission
- `Currency`: Native asset currency implementation (typically pallet_balances)
- `XcmSender`: XCM router for sending cross-chain messages
- `AssetHubLocation`: Constant location of Asset Hub (1, Parachain(1000))

## Runtime Integration

Add to `construct_runtime!` macro:

```rust
#[runtime::pallet_index(76)]
pub type TeleportXrt = pallet_robonomics_teleport;
```

Configure in runtime (typically in `lib.rs` or `xcm_config.rs`):

```rust
impl pallet_robonomics_teleport::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type XcmSender = XcmRouter;
    type AssetHubLocation = AssetHubLocation;
}
```

## How It Works

### XCM Message Flow

The `send` extrinsic constructs and sends an XCM message with the following instruction sequence:

1. **WithdrawAsset**
   - Withdraws native assets from the sender's account
   - Assets are placed in the holding register

2. **InitiateTransfer**
   - Teleports assets to Asset Hub with teleport semantics
   - Executes locally to burn assets on source chain
   - Sends XCM message to destination
   - Parameters:
     - `destination`: Asset Hub location (1, Parachain(1000))
     - `remote_fees`: Teleport filter for fee assets
     - `assets`: Teleport filter for transferred assets
     - `remote_xcm`: Instructions to execute on Asset Hub

3. **Remote XCM Execution on Asset Hub**
   - **PayFees**: Pays execution fees using relay chain asset
   - **DepositAsset**: Mints and deposits teleported assets to beneficiary

### Teleport Semantics

Teleport is a trust-based transfer mechanism where:
- Assets are **burned** on the source chain (Robonomics)
- Assets are **minted** on the destination chain (Asset Hub)
- Both chains must trust each other's asset issuance
- No additional proofs or confirmations required

### Fee Payment

Fees on Asset Hub are paid in **relay chain asset** (DOT/KSM):
- The `fee` parameter specifies relay asset amount
- Fees are independent of the teleported amount
- Sender must ensure Asset Hub account has relay tokens
- Failed fee payment will cause the transfer to fail

## Error Handling

The pallet implements comprehensive error handling:

| Error | Description | Cause |
|-------|-------------|-------|
| `AmountOverflow` | Amount exceeds u128 | Balance type larger than u128 |
| `InvalidAssetFilter` | Asset filter construction failed | Internal XCM configuration error |
| `SendFailure` | XCM message send failed | XCM router error or destination unreachable |

**Note:** Balance validation (insufficient funds) is performed during XCM local execution by InitiateTransfer, not in the extrinsic itself.

## Testing

### Unit Tests

Run the pallet unit tests:

```bash
cargo test -p pallet-robonomics-teleport
```

**Test Coverage:**
- ✅ Basic send functionality with event emission
- ✅ Send with maximum balance
- ✅ Send with different beneficiary addresses
- ✅ Send with varying fee amounts
- ✅ Genesis configuration validation

### Benchmarking

Run benchmarks to determine weight values:

```bash
cargo bench -p pallet-robonomics-teleport
```

The benchmark measures the computational cost of the `send` extrinsic including:
- XCM message construction
- Message validation and sending
- Event emission

### XCM Simulator Tests

For comprehensive integration testing with actual XCM message delivery between chains, see [XCM_SIMULATOR_TESTING.md](./XCM_SIMULATOR_TESTING.md).

**Simulator Test Scenarios:**
- Cross-chain message delivery validation
- Asset teleportation between parachains
- Fee handling and payment verification
- Edge cases (minimum balances, large amounts)
- Error conditions (insufficient balance, overflow)

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
- **No Fee Estimation**: Fee must be specified by caller
- **Trust Required**: Both chains must support teleport
- **No Retry Logic**: Failed transfers must be initiated again

## License

Apache-2.0
