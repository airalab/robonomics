# XCM Simulator Testing Guide for pallet-robonomics-teleport

This document provides guidance on implementing comprehensive XCM delivery tests using the [`xcm-simulator`](https://github.com/paritytech/polkadot-sdk/tree/master/polkadot/xcm/xcm-simulator) framework from polkadot-sdk.

## Overview

The `xcm-simulator` framework allows you to test XCM message delivery between parachains in a simulated multi-chain environment without requiring a full relay chain setup. This is essential for validating that the `pallet-robonomics-teleport` correctly handles XCM teleport operations between chains.

## Recommended Test Structure

### 1. Test Network Setup

Create a test network with:
- **Relay Chain**: Simulated relay chain for coordinating parachains
- **Parachain A (Source)**: Chain with `pallet-robonomics-teleport` installed
- **Parachain B (Destination)**: Target chain (Asset Hub simulation)

### 2. Test Scenarios

Implement the following test scenarios to validate different conditions:

#### Basic Functionality Tests

1. **Simple Teleport Test**
   - Test teleporting a small amount of native tokens
   - Verify balances decrease on source chain
   - Verify XCM message is constructed correctly
   - Check that the `Sent` event is emitted

2. **Large Amount Teleport**
   - Test with significant balance transfers
   - Validate proper handling of large u128 amounts
   - Ensure no overflow or underflow issues

3. **Multiple Sequential Teleports**
   - Execute multiple teleport operations from the same account
   - Verify each operation is independent
   - Check cumulative balance changes

#### Edge Cases

4. **Minimum Balance Teleport**
   - Test with amounts just above existential deposit
   - Verify proper handling of small balances

5. **Varying Fee Amounts**
   - Test with different fee parameters
   - Ensure fee is correctly passed to `PayFees` instruction
   - Validate that fees don't affect the teleported amount

6. **Different Beneficiaries**
   - Test with various AccountId32 beneficiary addresses
   - Verify correct encoding and delivery

#### Error Handling

7. **Insufficient Balance**
   - Attempt teleport with amount exceeding balance
   - Verify proper error handling

8. **Amount Overflow**
   - Test with balance amounts that exceed u128
   - Verify `AmountOverflow` error is returned

## Implementation Example

```rust
use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain, TestExt};

// Define your test network
decl_test_network! {
    pub struct TestNet {
        relay_chain = Relay,
        parachains = vec![
            (2000, SourceChain),
            (1000, AssetHub),
        ],
    }
}

// Implement test
#[test]
fn test_teleport_with_verification() {
    TestNet::reset();

    SourceChain::execute_with(|| {
        let amount = 1000u64;
        let fee = 100u64;
        let beneficiary = [1u8; 32];
        
        // Execute teleport
        assert_ok!(Teleport::send(
            RuntimeOrigin::signed(ALICE),
            beneficiary,
            amount,
            fee,
        ));
        
        // Verify event
        assert!(System::events().iter().any(|e| matches!(
            &e.event,
            RuntimeEvent::Teleport(Event::Sent { .. })
        )));
    });
    
    // Verify on destination
    AssetHub::execute_with(|| {
        // Check that XCM message was received and processed
        // Verify beneficiary balance increased
    });
}
```

## Integration with Existing Tests

The current `tests.rs` file provides unit tests with mocked XCM components. XCM simulator tests would complement these by providing integration-level validation with real XCM execution.

### Current Coverage

- ✅ Unit tests for pallet functionality
- ✅ Event emission verification  
- ✅ Basic parameter validation
- ⏳ Integration tests with xcm-simulator (to be implemented)

## Dependencies Required

Add to `Cargo.toml` dev-dependencies:

```toml
xcm-simulator = { version = "25.0.0" }
xcm-builder = { workspace = true }
polkadot-parachain-primitives = { workspace = true }
polkadot-runtime-parachains = { workspace = true }
```

Note: Ensure compatibility with your polkadot-sdk version.

## Running Tests

```bash
# Run unit tests
cargo test -p pallet-robonomics-teleport

# Run only simulator tests (when implemented)
cargo test -p pallet-robonomics-teleport simulator

# Run with output
cargo test -p pallet-robonomics-teleport -- --nocapture
```

## Benefits of XCM Simulator Testing

1. **Realistic Scenarios**: Tests actual XCM message construction and delivery
2. **Multi-Chain Validation**: Verify behavior across chain boundaries
3. **Fee Handling**: Test actual fee payment mechanisms
4. **Error Propagation**: Validate error handling in cross-chain context
5. **Regression Prevention**: Catch XCM-related regressions early

## Next Steps

1. Set up basic xcm-simulator test infrastructure
2. Implement core test scenarios listed above
3. Add edge case tests
4. Integrate with CI/CD pipeline
5. Document test results and coverage

## References

- [XCM Simulator Documentation](https://paritytech.github.io/polkadot-sdk/master/xcm_simulator/index.html)
- [XCM Simulator Examples](https://github.com/paritytech/polkadot-sdk/tree/master/polkadot/xcm/xcm-simulator/example)
- [Polkadot XCM Format](https://github.com/paritytech/polkadot-sdk/tree/master/polkadot/xcm)
- [XCM ExecuteXcm Trait](https://paritytech.github.io/polkadot-sdk/master/xcm_executor/trait.ExecuteXcm.html)

## Notes

- XCM simulator tests require careful configuration of runtime pallets
- Each parachain in the test network needs proper XCM configuration
- Asset transactors and barriers must be configured correctly
- Tests should validate both success and failure scenarios
- Consider testing with different XCM versions for compatibility
