# Robonomics XCM Teleport Pallet

A specialized pallet for teleporting native assets from the Robonomics parachain to the Asset Hub parachain using XCM (Cross-Consensus Messaging).

## Overview

This pallet provides a simplified and restricted version of XCM teleport functionality specifically designed for the Robonomics network. It enables users to teleport native XRT tokens to the Asset Hub parachain where they can be used for various cross-chain operations.

## Features

- **Single Asset Support**: Only supports the native asset (XRT via pallet_balances)
- **Asset-Hub Only**: Teleports are restricted to the Asset Hub parachain
- **Simplified Interface**: No separate fees parameter needed
- **Secure**: Validates all inputs and ensures proper XCM execution

## Usage

### Teleporting Assets

To teleport assets to Asset Hub:

```rust
use xcm::prelude::*;

let beneficiary = Box::new(VersionedLocation::V5(Location::new(
    0,
    [AccountId32 {
        network: None,
        id: recipient_account_id,
    }]
)));

let amount = 1_000_000_000; // Amount in native token (e.g., XRT)

RobonomicsTeleport::teleport_assets(
    origin,
    beneficiary,
    amount
)?;
```

## Configuration

The pallet requires the following configuration in your runtime:

```rust
parameter_types! {
    pub const AssetHubParaId: u32 = 1000;
    pub const RobonomicsTeleportPalletId: PalletId = PalletId(*b"robo/tel");
}

impl pallet_robonomics_teleport::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type XcmSender = XcmRouter;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type LocationToAccountId = LocationToAccountId;
    type AssetHubParaId = AssetHubParaId;
    type PalletId = RobonomicsTeleportPalletId;
}
```

## How It Works

The teleport process follows these steps:

1. **Validation**: The pallet validates:
   - The destination is Asset Hub (parachain 1000)
   - The asset is the native asset (Location::here())
   - The amount is greater than zero
   - The sender has sufficient balance

2. **XCM Message Construction**: An XCM message is built with:
   - `WithdrawAsset`: Withdraws assets from the holding register
   - `InitiateTeleport`: Starts the teleport to Asset Hub with:
     - `BuyExecution`: Pays for execution on Asset Hub
     - `DepositAsset`: Deposits assets to the beneficiary

3. **Local Execution**: The XCM message is executed locally to:
   - Withdraw the assets from the sender's account
   - Prepare them for teleportation

4. **Send to Destination**: The message is sent to Asset Hub where:
   - Execution fees are paid from the teleported assets
   - Remaining assets are deposited to the beneficiary

## Fees

Fees are handled automatically using the teleported assets:
- A portion of the teleported assets is used to pay for execution on Asset Hub
- The remaining assets are deposited to the beneficiary account
- The parachain's sovereign account balance on Asset Hub must have sufficient funds for the initial execution

## Security Considerations

- The pallet only allows teleportation to Asset Hub to prevent misuse
- Only native assets can be teleported (no foreign assets)
- All parameters are validated before execution
- Local XCM execution is checked for success before sending

## Testing

Run the pallet tests with:

```bash
cargo test -p pallet-robonomics-teleport
```

## License

Apache-2.0
