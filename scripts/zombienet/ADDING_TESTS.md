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
