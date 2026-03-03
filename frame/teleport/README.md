# Robonomics XCM Teleport Pallet (`TeleportXrt`)

A specialized pallet for sending native assets from the Robonomics parachain to the Asset Hub parachain using XCM (Cross-Consensus Messaging).

## Overview

This pallet provides a simplified and restricted version of XCM teleport functionality specifically designed for the Robonomics network. It enables users to send native XRT tokens to the Asset Hub parachain where they can be used for various cross-chain operations.

## Features

- **Single Asset Support**: Only supports the native asset (XRT via pallet_balances)
- **Asset Hub Only**: Transfers are restricted to the Asset Hub parachain
- **Simplified Interface**: Explicit fee parameter for fine-grained control
- **Modern XCM**: Uses XCM v5 instructions (InitiateTransfer, PayFees)
- **Secure**: Validates all inputs and ensures proper error handling

## Usage

### Sending Assets

To send assets to Asset Hub:

```rust
let beneficiary = [1u8; 32]; // Recipient AccountId32 on Asset Hub
let amount = 1_000_000_000; // Amount in native token (e.g., XRT)
let fee = 100_000; // Fee amount in relay chain asset

TeleportXrt::send(
    origin,
    beneficiary,
    amount,
    fee
)?;
```

## Configuration

The pallet requires the following configuration in your runtime:

```rust
parameter_types! {
    pub AssetHubLocation: Location = Location::new(1, [Parachain(1000)]);
}

impl pallet_robonomics_teleport::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type XcmSender = XcmRouter;
    type AssetHubLocation = AssetHubLocation;
}
```

## How It Works

The send process follows these steps:

1. **Validation**: The pallet validates:
   - The amount can be converted to u128
   - The asset transfer filter is valid

2. **XCM Message Construction**: An XCM message is built with:
   - `WithdrawAsset`: Withdraws assets from the sender
   - `InitiateTransfer`: Initiates transfer to Asset Hub with teleport semantics
   - `PayFees`: Pays for execution on Asset Hub using relay chain asset
   - `DepositAsset`: Deposits assets to the beneficiary

3. **Execution**: `InitiateTransfer` handles local execution automatically:
   - Withdraws the assets from the sender's account
   - Sends the XCM message to Asset Hub
   - No manual local execution needed

4. **Delivery**: The message is delivered to Asset Hub where:
   - Fees are paid using the relay chain asset
   - Assets are deposited to the beneficiary

## Fees

Fees are specified explicitly as a separate parameter:
- The `fee` parameter specifies the amount of relay chain asset to use for execution on Asset Hub
- Fees are independent of the teleported amount
- This provides fine-grained control over execution costs

## Security Considerations

- The pallet only allows sending to Asset Hub to prevent misuse
- Only native assets can be sent (no foreign assets)
- All parameters are validated before execution
- Proper error handling for amount overflow and invalid configurations
- No unwraps or panics in the code path

## Testing

Run the pallet tests with:

```bash
cargo test -p pallet-robonomics-teleport
```

### XCM Simulator Tests

For comprehensive XCM delivery testing using the xcm-simulator framework, see [XCM_SIMULATOR_TESTING.md](./XCM_SIMULATOR_TESTING.md).

The simulator tests validate:
- Cross-chain message delivery
- Asset teleportation between parachains
- Fee handling and payment
- Various edge cases and error conditions

## Runtime Integration

Add to your runtime's `construct_runtime!` macro:

```rust
#[runtime::pallet_index(76)]
pub type TeleportXrt = pallet_robonomics_teleport;
```

Configure in `xcm_config.rs`:

```rust
impl pallet_robonomics_teleport::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type XcmSender = XcmRouter;
    type AssetHubLocation = AssetHubLocation;
}
```

## License

Apache-2.0
