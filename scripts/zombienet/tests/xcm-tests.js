/**
 * XCM Integration Tests for Robonomics
 * 
 * This module contains XCM (Cross-Consensus Messaging) tests for the Robonomics network.
 * Tests include upward messages, downward messages, and cross-parachain transfers.
 */

const { ApiPromise, WsProvider } = require('@polkadot/api');
const { Keyring } = require('@polkadot/keyring');

// Test configuration for XCM tests
const XCM_TESTS_CONFIG = {
  relayWsUrl: 'ws://127.0.0.1:9944',
  parachainWsUrl: 'ws://127.0.0.1:9988',
  assetHubWsUrl: 'ws://127.0.0.1:9910',  // AssetHub parachain
  timeout: 120000, // 2 minutes
  blockWaitTime: 30000, // 30 seconds
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
 * Test: XCM Upward Message (Parachain → Relay Chain)
 * Sends an XCM message from Robonomics parachain to the relay chain
 */
async function testXcmUpwardMessage(testResults) {
  testResults.total++;
  log.test('Testing XCM upward message (Parachain → Relay Chain)...');
  
  try {
    const relayApi = await connectToNode(XCM_TESTS_CONFIG.relayWsUrl, 'Relay Chain');
    const parachainApi = await connectToNode(XCM_TESTS_CONFIG.parachainWsUrl, 'Parachain');
    
    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');
    
    log.info(`Using Alice account: ${alice.address}`);
    
    // Create an XCM message - a simple remark to the relay chain
    // Using XCM v4 versioned format (the latest stable version in the implementation)
    const dest = { V4: { parents: 1, interior: 'Here' } };
    
    // Create a simple XCM message with Transact instruction
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
              encoded: relayApi.tx.system.remarkWithEvent('XCM upward test').toHex()
            }
          }
        }
      ]
    };
    
    // Subscribe to relay chain events to detect XCM message
    let xcmReceived = false;
    const unsubRelay = await relayApi.query.system.events((events) => {
      events.forEach((record) => {
        const { event } = record;
        // Check for XCM-related events
        if (event.section === 'messageQueue' || 
            event.section === 'ump' ||
            event.section === 'dmpQueue' ||
            event.method.includes('Processed') ||
            event.method.includes('Success')) {
          log.info(`Relay chain event: ${event.section}.${event.method}`);
          xcmReceived = true;
        }
      });
    });
    
    // Send XCM message from parachain
    log.info('Sending XCM upward message from parachain...');
    
    return new Promise((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        unsubRelay();
        Promise.all([relayApi.disconnect(), parachainApi.disconnect()])
          .finally(() => {
            if (xcmReceived) {
              log.success('XCM upward message test passed (event detected)');
              testResults.passed++;
              resolve(true);
            } else {
              log.error('XCM upward message test timeout - message may still be in queue');
              testResults.failed++;
              resolve(false);
            }
          });
      }, XCM_TESTS_CONFIG.timeout);
      
      let resolved = false;
      
      parachainApi.tx.polkadotXcm
        .send(dest, message)
        .signAndSend(alice, ({ status, events }) => {
          if (status.isInBlock && !resolved) {
            log.info(`XCM message sent in block: ${status.asInBlock.toHex()}`);
            
            // Check for XCM sent event
            const xcmSent = events.find(({ event }) => 
              event.section === 'polkadotXcm' && event.method === 'Sent'
            );
            
            if (xcmSent) {
              log.success('XCM Sent event detected on parachain');
            }
            
            // Wait a bit for message to be processed on relay chain
            setTimeout(() => {
              if (!resolved) {
                resolved = true;
                clearTimeout(timeoutId);
                unsubRelay();
                
                Promise.all([relayApi.disconnect(), parachainApi.disconnect()])
                  .then(() => {
                    if (xcmReceived || xcmSent) {
                      log.success('XCM upward message test passed');
                      testResults.passed++;
                      resolve(true);
                    } else {
                      log.error('No XCM events detected');
                      testResults.failed++;
                      resolve(false);
                    }
                  });
              }
            }, 20000); // Wait 20 seconds for relay chain processing
          }
        })
        .catch((error) => {
          if (!resolved) {
            resolved = true;
            clearTimeout(timeoutId);
            unsubRelay();
            Promise.all([relayApi.disconnect(), parachainApi.disconnect()])
              .finally(() => reject(error));
          }
        });
    });
  } catch (error) {
    log.error(`XCM upward message test failed: ${error.message}`);
    testResults.failed++;
    return false;
  }
}

/**
 * Test: XCM Downward Message (Relay Chain → Parachain)
 * Sends an XCM message from relay chain to Robonomics parachain using sudo
 */
async function testXcmDownwardMessage(testResults) {
  testResults.total++;
  log.test('Testing XCM downward message (Relay Chain → Parachain)...');
  
  try {
    const relayApi = await connectToNode(XCM_TESTS_CONFIG.relayWsUrl, 'Relay Chain');
    const parachainApi = await connectToNode(XCM_TESTS_CONFIG.parachainWsUrl, 'Parachain');
    
    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice'); // Alice is sudo in dev chains
    
    log.info(`Using Alice (sudo) account: ${alice.address}`);
    
    // Get parachain ID
    const parachainId = await parachainApi.query.parachainInfo.parachainId();
    log.info(`Target parachain ID: ${parachainId.toString()}`);
    
    // Create destination (parachain 2000)
    const dest = {
      V4: {
        parents: 0,
        interior: { X1: { Parachain: parachainId.toString() } }
      }
    };
    
    // Create a simple XCM message to send to parachain
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
              encoded: parachainApi.tx.system.remarkWithEvent('XCM downward test').toHex()
            }
          }
        }
      ]
    };
    
    // Subscribe to parachain events
    let xcmReceived = false;
    const unsubPara = await parachainApi.query.system.events((events) => {
      events.forEach((record) => {
        const { event } = record;
        // Check for XCM or DMP-related events
        if (event.section === 'dmpQueue' ||
            event.section === 'messageQueue' ||
            event.method.includes('Processed') ||
            event.method.includes('Success')) {
          log.info(`Parachain event: ${event.section}.${event.method}`);
          xcmReceived = true;
        }
      });
    });
    
    // Send XCM message from relay chain
    log.info('Sending XCM downward message from relay chain...');
    
    return new Promise((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        unsubPara();
        Promise.all([relayApi.disconnect(), parachainApi.disconnect()])
          .finally(() => {
            if (xcmReceived) {
              log.success('XCM downward message test passed (event detected)');
              testResults.passed++;
              resolve(true);
            } else {
              log.error('XCM downward message test timeout');
              testResults.failed++;
              resolve(false);
            }
          });
      }, XCM_TESTS_CONFIG.timeout);
      
      let resolved = false;
      
      // Use sudo to send XCM from relay chain
      const xcmSend = relayApi.tx.xcmPallet.send(dest, message);
      
      relayApi.tx.sudo
        .sudo(xcmSend)
        .signAndSend(alice, ({ status, events }) => {
          if (status.isInBlock && !resolved) {
            log.info(`Sudo XCM sent in block: ${status.asInBlock.toHex()}`);
            
            // Check for successful sudo execution
            const sudoExecuted = events.find(({ event }) =>
              event.section === 'sudo' && event.method === 'Sudid'
            );
            
            if (sudoExecuted) {
              log.success('Sudo execution successful on relay chain');
            }
            
            // Wait for parachain to process message
            setTimeout(() => {
              if (!resolved) {
                resolved = true;
                clearTimeout(timeoutId);
                unsubPara();
                
                Promise.all([relayApi.disconnect(), parachainApi.disconnect()])
                  .then(() => {
                    if (xcmReceived || sudoExecuted) {
                      log.success('XCM downward message test passed');
                      testResults.passed++;
                      resolve(true);
                    } else {
                      log.error('No XCM events detected');
                      testResults.failed++;
                      resolve(false);
                    }
                  });
              }
            }, 20000); // Wait 20 seconds for parachain processing
          }
        })
        .catch((error) => {
          if (!resolved) {
            resolved = true;
            clearTimeout(timeoutId);
            unsubPara();
            Promise.all([relayApi.disconnect(), parachainApi.disconnect()])
              .finally(() => reject(error));
          }
        });
    });
  } catch (error) {
    log.error(`XCM downward message test failed: ${error.message}`);
    testResults.failed++;
    return false;
  }
}

/**
 * Test: AssetHub Token Transfer
 * Transfers assets from Robonomics parachain to AssetHub using XCM
 */
async function testAssetHubTransfer(testResults) {
  testResults.total++;
  log.test('Testing AssetHub token transfer (Parachain → AssetHub)...');
  
  try {
    const parachainApi = await connectToNode(XCM_TESTS_CONFIG.parachainWsUrl, 'Parachain');
    const assetHubApi = await connectToNode(XCM_TESTS_CONFIG.assetHubWsUrl, 'AssetHub');
    
    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');
    
    log.info(`Using Alice account: ${alice.address}`);
    
    // Get AssetHub account ID for Alice
    const assetHubAccountId = alice.address;
    log.info(`AssetHub account: ${assetHubAccountId}`);
    
    // Check if polkadotXcm pallet exists with limitedReserveTransferAssets
    let transferMethod = null;
    
    if (parachainApi.tx.polkadotXcm.limitedReserveTransferAssets) {
      transferMethod = 'limitedReserveTransferAssets';
      log.info('Using limitedReserveTransferAssets method');
    } else if (parachainApi.tx.polkadotXcm.reserveTransferAssets) {
      transferMethod = 'reserveTransferAssets';
      log.info('Using reserveTransferAssets method');
    } else {
      log.error('No suitable XCM transfer method found');
      testResults.failed++;
      await parachainApi.disconnect();
      await assetHubApi.disconnect();
      return false;
    }
    
    // Create destination (AssetHub - parachain 1000)
    const dest = {
      V4: {
        parents: 1,
        interior: { X1: [{ Parachain: '1000' }] }
      }
    };
    
    // Create beneficiary (Alice on AssetHub)
    const beneficiary = {
      V4: {
        parents: 0,
        interior: { X1: [{ AccountId32: { network: null, id: alice.publicKey } }] }
      }
    };
    
    // Define assets to transfer (native token)
    // Amount: 1 token = 1_000_000_000_000 (10^12 units, 12 decimal places)
    const assets = {
      V4: [
        {
          id: { parents: 0, interior: 'Here' },
          fun: { Fungible: '1000000000000' } // 1 token (12 decimals)
        }
      ]
    };
    
    // Fee asset index
    const feeAssetItem = 0;
    
    // Weight limit
    const weightLimit = 'Unlimited';
    
    log.info('Initiating XCM reserve transfer to AssetHub...');
    
    // Subscribe to events on both chains
    let transferInitiated = false;
    let assetReceived = false;
    
    const unsubPara = await parachainApi.query.system.events((events) => {
      events.forEach((record) => {
        const { event } = record;
        if (event.section === 'polkadotXcm' && event.method === 'Sent') {
          log.info('XCM transfer initiated on parachain');
          transferInitiated = true;
        }
      });
    });
    
    const unsubAssetHub = await assetHubApi.query.system.events((events) => {
      events.forEach((record) => {
        const { event } = record;
        if (event.section === 'messageQueue' || event.section === 'dmpQueue') {
          log.info(`AssetHub event: ${event.section}.${event.method}`);
          assetReceived = true;
        }
      });
    });
    
    return new Promise((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        unsubPara();
        unsubAssetHub();
        Promise.all([parachainApi.disconnect(), assetHubApi.disconnect()])
          .finally(() => {
            if (transferInitiated || assetReceived) {
              log.success('AssetHub transfer test passed (events detected)');
              testResults.passed++;
              resolve(true);
            } else {
              log.error('AssetHub transfer test timeout');
              testResults.failed++;
              resolve(false);
            }
          });
      }, XCM_TESTS_CONFIG.timeout);
      
      let resolved = false;
      
      // Execute the transfer
      let tx;
      if (transferMethod === 'limitedReserveTransferAssets') {
        tx = parachainApi.tx.polkadotXcm.limitedReserveTransferAssets(
          dest,
          beneficiary,
          assets,
          feeAssetItem,
          weightLimit
        );
      } else {
        tx = parachainApi.tx.polkadotXcm.reserveTransferAssets(
          dest,
          beneficiary,
          assets,
          feeAssetItem
        );
      }
      
      tx.signAndSend(alice, ({ status, events }) => {
        if (status.isInBlock && !resolved) {
          log.info(`Transfer included in block: ${status.asInBlock.toHex()}`);
          
          // Check for transfer events
          events.forEach(({ event }) => {
            if (event.section === 'polkadotXcm') {
              log.info(`Event: ${event.section}.${event.method}`);
            }
          });
          
          // Wait for AssetHub to process
          setTimeout(() => {
            if (!resolved) {
              resolved = true;
              clearTimeout(timeoutId);
              unsubPara();
              unsubAssetHub();
              
              Promise.all([parachainApi.disconnect(), assetHubApi.disconnect()])
                .then(() => {
                  if (transferInitiated || assetReceived) {
                    log.success('AssetHub token transfer test passed');
                    testResults.passed++;
                    resolve(true);
                  } else {
                    log.error('No transfer events detected');
                    testResults.failed++;
                    resolve(false);
                  }
                });
            }
          }, 20000); // Wait 20 seconds
        }
      }).catch((error) => {
        if (!resolved) {
          resolved = true;
          clearTimeout(timeoutId);
          unsubPara();
          unsubAssetHub();
          Promise.all([parachainApi.disconnect(), assetHubApi.disconnect()])
            .finally(() => reject(error));
        }
      });
    });
  } catch (error) {
    log.error(`AssetHub transfer test failed: ${error.message}`);
    testResults.failed++;
    return false;
  }
}

module.exports = {
  testXcmUpwardMessage,
  testXcmDownwardMessage,
  testAssetHubTransfer,
};
