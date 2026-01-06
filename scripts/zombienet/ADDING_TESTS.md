# Adding New Tests to the Zombienet Test Suite

This guide explains how to extend the existing test suite with new test cases.

## Test Structure

Each test function should follow this pattern:

```javascript
async function testNewFeature() {
  testResults.total++;
  log.test('Testing new feature...');
  
  try {
    // 1. Connect to the chain
    const api = await connectToNode(
      TESTS_CONFIG.parachainWsUrl, 
      'Parachain'
    );
    
    // 2. Perform test operations
    // Your test logic here
    
    // 3. Verify results
    // Add assertions
    
    // 4. Clean up
    await api.disconnect();
    
    log.success('New feature test passed');
    testResults.passed++;
    return true;
  } catch (error) {
    log.error(`New feature test failed: ${error.message}`);
    testResults.failed++;
    return false;
  }
}
```

## Example Tests

### Testing Custom Pallet Functions

For Robonomics-specific pallets (RWS, Launch, DataLog, etc.):

```javascript
async function testDatalogRecord() {
  testResults.total++;
  log.test('Testing Datalog record submission...');
  
  try {
    const api = await connectToNode(TESTS_CONFIG.parachainWsUrl, 'Parachain');
    const { Keyring } = require('@polkadot/keyring');
    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');
    
    // Submit a datalog record
    const record = api.createType('Vec<u8>', 'Test datalog entry');
    const tx = api.tx.datalog.record(record);
    
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('Timeout')), 60000);
      
      tx.signAndSend(alice, ({ status, events }) => {
        if (status.isInBlock) {
          clearTimeout(timeout);
          
          // Check for success event
          const success = events.find(({ event }) => 
            api.events.datalog.NewRecord.is(event)
          );
          
          if (success) {
            log.success('Datalog record submitted successfully');
            api.disconnect().then(() => {
              testResults.passed++;
              resolve(true);
            });
          } else {
            throw new Error('NewRecord event not found');
          }
        }
      }).catch(reject);
    });
  } catch (error) {
    log.error(`Datalog test failed: ${error.message}`);
    testResults.failed++;
    return false;
  }
}
```

### Testing Account Operations

```javascript
async function testBalanceTransfer() {
  testResults.total++;
  log.test('Testing balance transfer...');
  
  try {
    const api = await connectToNode(TESTS_CONFIG.parachainWsUrl, 'Parachain');
    const { Keyring } = require('@polkadot/keyring');
    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');
    const bob = keyring.addFromUri('//Bob');
    
    // Get initial balances
    const { data: initialBalance } = await api.query.system.account(bob.address);
    log.info(`Bob's initial balance: ${initialBalance.free.toString()}`);
    
    // Transfer amount
    const amount = api.createType('Balance', '1000000000000');
    
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('Timeout')), 60000);
      
      api.tx.balances.transfer(bob.address, amount)
        .signAndSend(alice, async ({ status, events }) => {
          if (status.isInBlock) {
            clearTimeout(timeout);
            
            // Verify new balance
            const { data: newBalance } = await api.query.system.account(bob.address);
            const balanceIncreased = newBalance.free.gt(initialBalance.free);
            
            if (balanceIncreased) {
              log.success('Balance transfer successful');
              log.info(`Bob's new balance: ${newBalance.free.toString()}`);
              await api.disconnect();
              testResults.passed++;
              resolve(true);
            } else {
              throw new Error('Balance did not increase');
            }
          }
        }).catch(reject);
    });
  } catch (error) {
    log.error(`Balance transfer test failed: ${error.message}`);
    testResults.failed++;
    return false;
  }
}
```

### Testing Query Operations

```javascript
async function testQueryRuntimeVersion() {
  testResults.total++;
  log.test('Testing runtime version query...');
  
  try {
    const api = await connectToNode(TESTS_CONFIG.parachainWsUrl, 'Parachain');
    
    // Query runtime version
    const version = await api.rpc.state.getRuntimeVersion();
    
    log.info(`Runtime spec name: ${version.specName.toString()}`);
    log.info(`Runtime spec version: ${version.specVersion.toString()}`);
    log.info(`Runtime impl version: ${version.implVersion.toString()}`);
    
    // Verify version info exists
    if (!version.specName || !version.specVersion) {
      throw new Error('Runtime version information incomplete');
    }
    
    await api.disconnect();
    
    log.success('Runtime version query test passed');
    testResults.passed++;
    return true;
  } catch (error) {
    log.error(`Runtime version query test failed: ${error.message}`);
    testResults.failed++;
    return false;
  }
}
```

### Testing Chain State

```javascript
async function testChainState() {
  testResults.total++;
  log.test('Testing chain state queries...');
  
  try {
    const api = await connectToNode(TESTS_CONFIG.parachainWsUrl, 'Parachain');
    
    // Get chain metadata
    const metadata = await api.rpc.state.getMetadata();
    log.info(`Metadata size: ${metadata.toHex().length / 2} bytes`);
    
    // Get current block hash
    const hash = await api.rpc.chain.getBlockHash();
    log.info(`Current block hash: ${hash.toHex()}`);
    
    // Get current timestamp
    const now = await api.query.timestamp.now();
    log.info(`Current timestamp: ${now.toString()}`);
    
    // Verify all queries returned valid data
    if (!metadata || !hash || !now) {
      throw new Error('Chain state query returned invalid data');
    }
    
    await api.disconnect();
    
    log.success('Chain state query test passed');
    testResults.passed++;
    return true;
  } catch (error) {
    log.error(`Chain state test failed: ${error.message}`);
    testResults.failed++;
    return false;
  }
}
```

## Integrating New Tests

1. Add your test function to `tests/integration-tests.js`

2. Call it in the `runTests()` function:

```javascript
async function runTests() {
  log.info('Starting Robonomics Zombienet Integration Tests');
  log.info('='.repeat(50));
  
  try {
    await sleep(30000); // Wait for network stabilization
    
    // Existing tests
    await testNetworkInitialization();
    await testBlockProduction();
    await testExtrinsicSubmission();
    
    // Add your new tests here
    await testDatalogRecord();
    await testBalanceTransfer();
    await testQueryRuntimeVersion();
    await testChainState();
    
    // Print results
    printResults();
  } catch (error) {
    handleError(error);
  }
}
```

3. Test locally before committing:

```bash
cd scripts/zombienet
./run-tests.sh
```

## Best Practices

1. **Isolate Tests**: Each test should be independent
2. **Clean Up**: Always disconnect API connections
3. **Handle Timeouts**: Set reasonable timeouts for async operations
4. **Log Progress**: Use the logger utility for consistent output
5. **Error Messages**: Provide clear error messages for debugging
6. **Test Data**: Use development accounts (Alice, Bob, etc.)
7. **Assertions**: Verify expected outcomes explicitly

## Common Patterns

### Waiting for Events

```javascript
const events = await new Promise((resolve, reject) => {
  const timeout = setTimeout(() => reject(new Error('Timeout')), 60000);
  const unsub = api.tx.someCall()
    .signAndSend(signer, ({ events = [], status }) => {
      if (status.isInBlock) {
        clearTimeout(timeout);
        unsub();
        resolve(events);
      }
    });
});
```

### Querying Storage

```javascript
// Single query
const value = await api.query.module.storage(key);

// Multiple queries
const [value1, value2] = await Promise.all([
  api.query.module.storage1(key1),
  api.query.module.storage2(key2),
]);

// Subscription
const unsub = await api.query.module.storage(key, (value) => {
  console.log('Value updated:', value.toString());
});
```

### Using Sudo

```javascript
const { Keyring } = require('@polkadot/keyring');
const keyring = new Keyring({ type: 'sr25519' });
const sudo = keyring.addFromUri('//Alice'); // Alice is sudo in dev chains

const tx = api.tx.someModule.privilegedCall(...args);
await api.tx.sudo.sudo(tx).signAndSend(sudo);
```

## XCM Testing Examples

The test suite includes XCM (Cross-Consensus Messaging) v4 tests demonstrating cross-chain communication patterns.

### Testing XCM Upward Messages (Parachain → Relay Chain)

```javascript
async function testXcmUpwardMessage(testResults) {
  testResults.total++;
  log.test('Testing XCM upward message...');
  
  try {
    const relayApi = await connectToNode(XCM_TESTS_CONFIG.relayWsUrl, 'Relay Chain');
    const parachainApi = await connectToNode(XCM_TESTS_CONFIG.parachainWsUrl, 'Parachain');
    
    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');
    
    // XCM v5 uses Location instead of MultiLocation
    const dest = { V4: { parents: 1, interior: 'Here' } };
    
    // Create XCM message
    const message = {
      V4: [
        {
          UnpaidExecution: {
            weight_limit: 'Unlimited'
          }
        },
        {
          Transact: {
            origin_kind: 'SovereignAccount',
            call: {
              encoded: parachainApi.tx.system.remarkWithEvent('XCM test').toHex()
            }
          }
        }
      ]
    };
    
    // Send XCM message
    await parachainApi.tx.polkadotXcm.send(dest, message)
      .signAndSend(alice, ({ status }) => {
        if (status.isInBlock) {
          log.info('XCM message sent');
        }
      });
    
    // Monitor for events on relay chain
    // ... event monitoring logic
    
    await relayApi.disconnect();
    await parachainApi.disconnect();
    
    log.success('XCM upward message test passed');
    testResults.passed++;
    return true;
  } catch (error) {
    log.error(`XCM upward message test failed: ${error.message}`);
    testResults.failed++;
    return false;
  }
}
```

### Testing XCM Downward Messages (Relay Chain → Parachain)

```javascript
async function testXcmDownwardMessage(testResults) {
  testResults.total++;
  log.test('Testing XCM downward message...');
  
  try {
    const relayApi = await connectToNode(XCM_TESTS_CONFIG.relayWsUrl, 'Relay Chain');
    const parachainApi = await connectToNode(XCM_TESTS_CONFIG.parachainWsUrl, 'Parachain');
    
    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');
    
    // Get parachain ID
    const parachainId = await parachainApi.query.parachainInfo.parachainId();
    
    // Destination: target parachain
    const dest = {
      V4: {
        parents: 0,
        interior: { X1: [{ Parachain: parachainId.toString() }] }
      }
    };
    
    // XCM message with sudo origin
    const message = {
      V4: [
        {
          UnpaidExecution: {
            weight_limit: 'Unlimited'
          }
        },
        {
          Transact: {
            origin_kind: 'Superuser',
            call: {
              encoded: parachainApi.tx.system.remarkWithEvent('Downward XCM').toHex()
            }
          }
        }
      ]
    };
    
    // Send via sudo from relay chain
    const xcmSend = relayApi.tx.xcmPallet.send(dest, message);
    await relayApi.tx.sudo.sudo(xcmSend)
      .signAndSend(alice, ({ status }) => {
        if (status.isInBlock) {
          log.info('Downward XCM sent via sudo');
        }
      });
    
    // Monitor DMP queue on parachain
    // ... event monitoring logic
    
    await relayApi.disconnect();
    await parachainApi.disconnect();
    
    log.success('XCM downward message test passed');
    testResults.passed++;
    return true;
  } catch (error) {
    log.error(`XCM downward message test failed: ${error.message}`);
    testResults.failed++;
    return false;
  }
}
```

### Testing Cross-Parachain Asset Transfers

```javascript
async function testAssetHubTransfer(testResults) {
  testResults.total++;
  log.test('Testing AssetHub token transfer...');
  
  try {
    const parachainApi = await connectToNode(XCM_TESTS_CONFIG.parachainWsUrl, 'Parachain');
    const assetHubApi = await connectToNode(XCM_TESTS_CONFIG.assetHubWsUrl, 'AssetHub');
    
    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');
    
    // Destination: AssetHub (parachain 1000)
    const dest = {
      V4: {
        parents: 1,
        interior: { X1: [{ Parachain: '1000' }] }
      }
    };
    
    // Beneficiary on AssetHub
    const beneficiary = {
      V4: {
        parents: 0,
        interior: { X1: [{ AccountId32: { network: null, id: alice.publicKey } }] }
      }
    };
    
    // Assets to transfer (native token)
    const assets = {
      V4: [
        {
          id: { parents: 0, interior: 'Here' },
          fun: { Fungible: '1000000000000' } // Amount
        }
      ]
    };
    
    // Execute reserve transfer
    await parachainApi.tx.polkadotXcm.limitedReserveTransferAssets(
      dest,
      beneficiary,
      assets,
      0, // Fee asset item
      'Unlimited' // Weight limit
    ).signAndSend(alice, ({ status }) => {
      if (status.isInBlock) {
        log.info('Asset transfer initiated');
      }
    });
    
    // Verify on AssetHub
    // ... balance verification logic
    
    await parachainApi.disconnect();
    await assetHubApi.disconnect();
    
    log.success('AssetHub transfer test passed');
    testResults.passed++;
    return true;
  } catch (error) {
    log.error(`AssetHub transfer test failed: ${error.message}`);
    testResults.failed++;
    return false;
  }
}
```

### Key Points for XCM Tests

1. **Use XCM v4 Types**: Use the `Location` type (v4+) instead of the legacy v3 `MultiLocation`; the examples above use XCM v4 enums (e.g. `V4` variants).
2. **Event Monitoring**: Subscribe to events on both source and destination chains
3. **Timeouts**: XCM messages may take time to process; use appropriate timeouts
4. **Weight Limits**: Specify weight limits for execution (Unlimited for testing)
5. **Origin Kinds**: Use appropriate origin kinds (SovereignAccount, Superuser, etc.)
6. **Error Handling**: XCM errors may occur in events; check for failure events

## Testing Tips

- Run tests multiple times to catch intermittent failures
- Increase log levels for debugging: `-lparachain=trace`
- Check zombienet output directory for node logs
- Use Polkadot.js Apps to manually verify chain state
- Consider adding test setup/teardown functions
- Document expected behavior for each test

## Resources

- [Polkadot.js API Docs](https://polkadot.js.org/docs/api)
- [Substrate Metadata](https://docs.substrate.io/reference/metadata/)
- [Testing Best Practices](https://wiki.polkadot.network/docs/build-integration)
- [XCM Format Documentation](https://wiki.polkadot.network/docs/learn-xcm)
- [XCM v5 Migration Guide](https://github.com/paritytech/polkadot-sdk/blob/master/docs/XCM_v5_MIGRATION.md)
