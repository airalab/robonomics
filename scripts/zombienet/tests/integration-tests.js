#!/usr/bin/env node

/**
 * Zombienet Integration Tests for Robonomics
 * 
 * This script runs basic integration tests on a Robonomics network
 * spawned using zombienet.
 */

const { ApiPromise, WsProvider } = require('@polkadot/api');

// Test configuration
const TESTS_CONFIG = {
  relayWsUrl: 'ws://127.0.0.1:9944',
  parachainWsUrl: 'ws://127.0.0.1:9988',
  timeout: 300000, // 5 minutes
  blockProductionWaitTime: 60000, // 1 minute
  transactionTimeout: 60000, // 1 minute
  networkStabilizationTime: 30000, // 30 seconds
};

// Test results tracking
let testResults = {
  passed: 0,
  failed: 0,
  total: 0,
};

/**
 * Logger utility
 */
const log = {
  info: (msg) => console.log(`[INFO] ${msg}`),
  success: (msg) => console.log(`[✓] ${msg}`),
  error: (msg) => console.error(`[✗] ${msg}`),
  test: (msg) => console.log(`[TEST] ${msg}`),
};

/**
 * Sleep utility
 */
const sleep = (ms) => new Promise(resolve => setTimeout(resolve, ms));

/**
 * Connect to a node via WebSocket
 */
async function connectToNode(wsUrl, nodeName) {
  log.info(`Connecting to ${nodeName} at ${wsUrl}...`);
  const provider = new WsProvider(wsUrl);
  const api = await ApiPromise.create({ provider });
  await api.isReady;
  log.success(`Connected to ${nodeName}`);
  return api;
}

/**
 * Test: Network initialization
 * Verifies that both relay chain and parachain nodes are accessible
 */
async function testNetworkInitialization() {
  testResults.total++;
  log.test('Testing network initialization...');
  
  try {
    // Connect to relay chain
    const relayApi = await connectToNode(TESTS_CONFIG.relayWsUrl, 'Relay Chain');
    const relayChainInfo = await relayApi.rpc.system.chain();
    log.info(`Relay chain: ${relayChainInfo.toString()}`);
    
    // Connect to parachain
    const parachainApi = await connectToNode(TESTS_CONFIG.parachainWsUrl, 'Parachain');
    const parachainInfo = await parachainApi.rpc.system.chain();
    log.info(`Parachain: ${parachainInfo.toString()}`);
    
    // Disconnect
    await relayApi.disconnect();
    await parachainApi.disconnect();
    
    log.success('Network initialization test passed');
    testResults.passed++;
    return true;
  } catch (error) {
    log.error(`Network initialization test failed: ${error.message}`);
    testResults.failed++;
    return false;
  }
}

/**
 * Test: Block production
 * Verifies that blocks are being produced on both chains
 */
async function testBlockProduction() {
  testResults.total++;
  log.test('Testing block production...');
  
  try {
    const relayApi = await connectToNode(TESTS_CONFIG.relayWsUrl, 'Relay Chain');
    const parachainApi = await connectToNode(TESTS_CONFIG.parachainWsUrl, 'Parachain');
    
    // Get initial block numbers
    const relayBlock1 = await relayApi.rpc.chain.getBlock();
    const parachainBlock1 = await parachainApi.rpc.chain.getBlock();
    
    const relayBlockNum1 = relayBlock1.block.header.number.toNumber();
    const parachainBlockNum1 = parachainBlock1.block.header.number.toNumber();
    
    log.info(`Relay chain initial block: #${relayBlockNum1}`);
    log.info(`Parachain initial block: #${parachainBlockNum1}`);
    
    // Wait for block production
    log.info(`Waiting ${TESTS_CONFIG.blockProductionWaitTime / 1000}s for block production...`);
    await sleep(TESTS_CONFIG.blockProductionWaitTime);
    
    // Get new block numbers
    const relayBlock2 = await relayApi.rpc.chain.getBlock();
    const parachainBlock2 = await parachainApi.rpc.chain.getBlock();
    
    const relayBlockNum2 = relayBlock2.block.header.number.toNumber();
    const parachainBlockNum2 = parachainBlock2.block.header.number.toNumber();
    
    log.info(`Relay chain new block: #${relayBlockNum2}`);
    log.info(`Parachain new block: #${parachainBlockNum2}`);
    
    // Verify blocks increased
    if (relayBlockNum2 <= relayBlockNum1) {
      throw new Error('Relay chain is not producing blocks');
    }
    
    if (parachainBlockNum2 <= parachainBlockNum1) {
      throw new Error('Parachain is not producing blocks');
    }
    
    await relayApi.disconnect();
    await parachainApi.disconnect();
    
    log.success('Block production test passed');
    testResults.passed++;
    return true;
  } catch (error) {
    log.error(`Block production test failed: ${error.message}`);
    testResults.failed++;
    return false;
  }
}

/**
 * Test: Basic extrinsic submission
 * Submits a remark extrinsic and verifies it's included in a block
 */
async function testExtrinsicSubmission() {
  testResults.total++;
  log.test('Testing extrinsic submission...');
  
  try {
    const parachainApi = await connectToNode(TESTS_CONFIG.parachainWsUrl, 'Parachain');
    
    // Get Alice's account (dev account with funds)
    const { Keyring } = require('@polkadot/keyring');
    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');
    
    log.info(`Using Alice account: ${alice.address}`);
    
    // Create a remark extrinsic
    const remark = 'Zombienet integration test';
    const tx = parachainApi.tx.system.remark(remark);
    
    // Submit the transaction
    log.info('Submitting remark extrinsic...');
    
    return new Promise((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        parachainApi.disconnect()
          .catch(() => {}) // Ignore disconnect errors in timeout
          .finally(() => {
            reject(new Error('Transaction timeout'));
          });
      }, TESTS_CONFIG.transactionTimeout);
      
      let resolved = false;
      
      tx.signAndSend(alice, ({ status, events }) => {
        if (status.isInBlock && !resolved) {
          resolved = true;
          log.info(`Transaction included in block: ${status.asInBlock.toHex()}`);
          clearTimeout(timeoutId);
          
          parachainApi.disconnect().then(() => {
            log.success('Extrinsic submission test passed');
            testResults.passed++;
            resolve(true);
          });
        } else if (status.isFinalized) {
          log.info(`Transaction finalized: ${status.asFinalized.toHex()}`);
        }
      }).catch((error) => {
        if (!resolved) {
          resolved = true;
          clearTimeout(timeoutId);
          parachainApi.disconnect().then(() => {
            reject(error);
          }).catch(() => {
            reject(error);
          });
        }
      });
    });
  } catch (error) {
    log.error(`Extrinsic submission test failed: ${error.message}`);
    testResults.failed++;
    return false;
  }
}

/**
 * Main test runner
 */
async function runTests() {
  log.info('Starting Robonomics Zombienet Integration Tests');
  log.info('='.repeat(50));
  
  try {
    // Wait a bit for the network to stabilize
    log.info('Waiting for network to stabilize...');
    await sleep(TESTS_CONFIG.networkStabilizationTime);
    
    // Run tests sequentially
    await testNetworkInitialization();
    await testBlockProduction();
    await testExtrinsicSubmission();
    
    // Print results
    log.info('='.repeat(50));
    log.info('Test Results:');
    log.info(`  Total:  ${testResults.total}`);
    log.info(`  Passed: ${testResults.passed}`);
    log.info(`  Failed: ${testResults.failed}`);
    log.info('='.repeat(50));
    
    if (testResults.failed > 0) {
      log.error('Some tests failed');
      process.exit(1);
    } else {
      log.success('All tests passed!');
      process.exit(0);
    }
  } catch (error) {
    log.error(`Test execution failed: ${error.message}`);
    console.error(error);
    process.exit(1);
  }
}

// Run tests if this script is executed directly
if (require.main === module) {
  runTests().catch((error) => {
    log.error(`Fatal error: ${error.message}`);
    console.error(error);
    process.exit(1);
  });
}

module.exports = { runTests };
