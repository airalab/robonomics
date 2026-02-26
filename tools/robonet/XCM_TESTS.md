# XCM Tests Documentation

## Overview

This document describes the comprehensive XCM (Cross-Consensus Messaging) tests implemented for the Robonomics parachain using the robonet tool.

## Architecture

### Network Topology

The tests support two topology modes:

1. **Simple**: Relay chain + Robonomics parachain
2. **AssetHub**: Relay chain + Robonomics parachain + AssetHub parachain (with HRMP channels)

### Test Categories

#### 1. XCM Upward Message Test (`test_xcm_upward_message`)

**Purpose**: Verify parachain can send XCM messages to relay chain via UMP (Upward Message Passing).

**Test Flow**:
- Connect to both parachain and relay chain
- Construct a simple XCM message
- Send message using `PolkadotXcm::send` extrinsic
- Monitor for XCM events
- Verify message structure and submission

**Key Features**:
- Tests UMP queue functionality
- Validates XCM message construction
- Checks event emission

#### 2. XCM Downward Message Test (`test_xcm_downward_message`)

**Purpose**: Verify relay chain can send XCM messages to parachain via DMP (Downward Message Passing).

**Test Flow**:
- Connect to relay chain and parachain
- Query parachain's relay block tracking
- Verify DMP queue infrastructure
- Validate parachain synchronization with relay chain

**Key Features**:
- Tests DMP queue operational status
- Verifies parachain relay chain tracking
- Validates message reception infrastructure

**Note**: Full downward message sending requires relay chain sudo access, which is not available in test environment. The test validates infrastructure readiness.

#### 3. Asset Teleportation Tests (`test_xcm_token_teleport`)

**Purpose**: Comprehensive testing of native asset (XRT) teleportation between Robonomics parachain and AssetHub.

**Test Suite**:

##### 3.1. Teleport to AssetHub (`test_teleport_to_assethub`)

**Flow**:
1. Connect to Robonomics parachain and AssetHub
2. Query initial balance on Robonomics (Alice's account)
3. Construct XCM destination (AssetHub location)
4. Construct XCM beneficiary (Alice's account on destination)
5. Construct XCM assets (native XRT token, 1 XRT = 1,000,000,000 COASE)
6. Execute `PolkadotXcm::limited_teleport_assets` extrinsic
7. Monitor XCM events:
   - `PolkadotXcm::Attempted` - XCM execution attempt
   - `PolkadotXcm::Sent` - XCM message sent
   - `XcmpQueue::XcmpMessageSent` - XCMP queue confirmation
8. Wait for cross-chain message processing
9. Query final balance on Robonomics
10. Verify balance decreased by teleport amount + fees

**Validations**:
- ✅ XCM message construction
- ✅ Transaction execution
- ✅ Event emission
- ✅ Balance changes
- ✅ Cross-chain communication via XCMP

##### 3.2. Teleport from AssetHub (`test_teleport_from_assethub`)

**Flow**:
1. Connect to AssetHub and Robonomics
2. Use Bob's account as sender
3. Construct reverse teleport (AssetHub → Robonomics)
4. Attempt teleport execution
5. Handle expected failure (Bob may not have assets on AssetHub)
6. Validate message structure

**Purpose**:
- Demonstrates bidirectional teleportation capability
- Validates XCM message construction for reverse flow
- Tests foreign asset representation

**Note**: This test may fail due to insufficient balance, but validates the XCM message structure and flow.

## Technical Details

### XCM Configuration

The Robonomics runtime has the following XCM configuration:

```rust
pub const ASSET_HUB_ID: u32 = 1000;
pub const PARA_ID: u32 = 2000;

// Teleport Trust Configuration
AssetHubTrustedTeleporter: (NativeAssetFilter, AssetHubLocation)
XcmTeleportFilter: Everything
XcmReserveTransferFilter: Nothing  // Reserve transfers disabled
```

**Key Points**:
- ✅ AssetHub (parachain 1000) is trusted for native asset teleportation
- ✅ Only native currency (XRT) can be teleported
- ❌ Reserve transfers are disabled
- ❌ Foreign assets not supported (no `pallet_assets`)

### XCM Message Structure

#### Destination Format (AssetHub)
```rust
Location {
    parents: 1,  // Go up to relay chain
    interior: X1(Parachain(1000))  // Target AssetHub
}
```

#### Beneficiary Format
```rust
Location {
    parents: 0,  // Same level
    interior: X1(AccountId32 {
        network: None,
        id: [account_bytes]
    })
}
```

#### Asset Format (Native)
```rust
MultiAsset {
    id: Concrete(Location {
        parents: 0,
        interior: Here  // Native asset
    }),
    fun: Fungible(amount_in_coase)
}
```

### Event Monitoring

The tests monitor for these XCM-related events:

1. **PolkadotXcm Pallet Events**:
   - `Attempted { outcome }` - XCM execution result
   - `Sent { origin, destination, message, message_id }` - XCM message sent
   - `AssetsTrapped { hash, origin, assets }` - Assets trapped (error case)

2. **XcmpQueue Pallet Events**:
   - `XcmpMessageSent { message_hash }` - XCMP message queued
   - `Fail { message_hash, error, weight }` - XCMP message failed

## Running the Tests

### Simple Topology (Basic XCM Tests)
```bash
robonet test --topology simple xcm_upward xcm_downward
```

### AssetHub Topology (Full XCM Teleport Tests)
```bash
robonet test --topology assethub
```

### Run Specific Test
```bash
robonet test --topology assethub xcm_teleport
```

### With JSON Output
```bash
robonet test --topology assethub --output json
```

## Test Requirements

### Network Requirements
- Relay chain (Rococo local)
- Robonomics parachain (Para ID: 2000)
- AssetHub parachain (Para ID: 1000) - for teleport tests
- HRMP channels established between parachains

### Account Requirements
- Alice account with balance on Robonomics
- Bob account (optional, for reverse teleport test)

### Infrastructure Requirements
- UMP queue operational (parachain → relay)
- DMP queue operational (relay → parachain)
- XCMP queue operational (parachain ↔ parachain)
- HRMP channels open (for AssetHub tests)

## Limitations and Future Work

### Current Limitations

1. **Foreign Asset Registration**: Not supported as Robonomics runtime lacks `pallet_assets`
2. **Reserve Transfers**: Disabled in runtime configuration
3. **Relay Chain Sudo**: Test environment lacks relay sudo for full DMP testing
4. **Multi-Asset XCM**: Only native currency supported

### Future Enhancements

1. **Extended XCM Instructions**: Test more complex XCM programs
2. **Fee Calculation Tests**: Verify XCM fee computation
3. **Location Conversion Tests**: Test `LocationToAccountId` conversions
4. **Error Handling**: More comprehensive error scenario testing
5. **Performance Tests**: Measure XCM execution times and gas costs

## Troubleshooting

### Test Failures

#### "Failed to connect to parachain/relay"
- Ensure network is spawned before tests
- Check websocket endpoints are correct
- Verify chains are producing blocks

#### "Teleport transaction failed"
- Check account has sufficient balance
- Verify HRMP channels are established
- Ensure AssetHub topology is used

#### "XCM events not found"
- This is expected in some test scenarios
- Tests validate message construction even if execution fails
- Check logs for detailed error messages

### Common Issues

1. **Insufficient Balance**: Tests require accounts with XRT balance
2. **Missing AssetHub**: Teleport tests require `--topology assethub`
3. **Network Timeout**: Initial network spawn may take 30-60 seconds
4. **XCMP Queue Full**: Rarely, message queue may be congested

## References

- [Polkadot XCM Documentation](https://wiki.polkadot.network/docs/learn-xcm)
- [Polkadot Foreign Asset Registration](https://docs.polkadot.com/chain-interactions/token-operations/register-foreign-asset/)
- [XCM Format Specification](https://github.com/paritytech/xcm-format)
- [Cumulus Documentation](https://github.com/paritytech/cumulus)

## Security Considerations

### Test Safety

1. **Local Environment Only**: Tests run on local test networks
2. **Development Accounts**: Uses well-known dev accounts (Alice, Bob)
3. **No Real Assets**: All tokens are test tokens
4. **Isolated Network**: No connection to production networks

### XCM Security Notes

1. **Weight Limits**: Tests use `Unlimited` weight for simplicity
   - Production should use calculated weight limits
2. **Trust Configuration**: Tests rely on AssetHub trust
   - Verify trust relationships in production
3. **Fee Payment**: Native asset used for XCM fees
   - Ensure sufficient balance for fee payment

## Code Structure

```
tools/robonet/src/tests/
├── mod.rs              # Test runner and infrastructure
├── xcm.rs              # XCM tests (this implementation)
├── network.rs          # Basic network tests
├── cps.rs              # CPS pallet tests
└── claim.rs            # Claim pallet tests
```

## Contributing

When adding new XCM tests:

1. Follow existing test patterns
2. Add comprehensive documentation
3. Include error handling
4. Test with both topologies where applicable
5. Update this documentation

## Version History

- **v1.0.0** (2026-02-26): Initial implementation
  - XCM upward message tests
  - XCM downward message tests
  - Native asset teleportation tests
  - Comprehensive documentation
