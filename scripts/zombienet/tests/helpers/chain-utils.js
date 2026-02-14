/**
 * Chain Utilities for Zombienet Tests
 * 
 * Common utilities for interacting with blockchain nodes in tests.
 */

const { ApiPromise, WsProvider } = require('@polkadot/api');

/**
 * Logger utility
 */
const log = {
  info: (msg) => console.log(`[INFO] ${msg}`),
  success: (msg) => console.log(`[✓] ${msg}`),
  error: (msg) => console.error(`[✗] ${msg}`),
  warn: (msg) => console.warn(`[⚠] ${msg}`),
  debug: (msg) => console.log(`[DEBUG] ${msg}`),
  test: (msg) => console.log(`[TEST] ${msg}`),
};

/**
 * Sleep utility
 * @param {number} ms - Milliseconds to sleep
 * @returns {Promise<void>}
 */
const sleep = (ms) => new Promise(resolve => setTimeout(resolve, ms));

/**
 * Connect to a blockchain node via WebSocket
 * @param {string} wsUrl - WebSocket URL
 * @param {string} nodeName - Human-readable node name for logging
 * @param {number} maxRetries - Maximum connection attempts
 * @param {number} retryDelay - Delay between retries in ms
 * @returns {Promise<ApiPromise>} - Connected API instance
 */
async function connectToNode(wsUrl, nodeName, maxRetries = 3, retryDelay = 5000) {
  let lastError;
  
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      log.info(`Connecting to ${nodeName} at ${wsUrl} (attempt ${attempt}/${maxRetries})...`);
      const provider = new WsProvider(wsUrl);
      const api = await ApiPromise.create({ provider });
      await api.isReady;
      log.success(`Connected to ${nodeName}`);
      return api;
    } catch (error) {
      lastError = error;
      log.warn(`Failed to connect to ${nodeName}: ${error.message}`);
      
      if (attempt < maxRetries) {
        log.info(`Retrying in ${retryDelay / 1000}s...`);
        await sleep(retryDelay);
      }
    }
  }
  
  throw new Error(`Failed to connect to ${nodeName} after ${maxRetries} attempts: ${lastError.message}`);
}

/**
 * Get the current block number
 * @param {ApiPromise} api - API instance
 * @returns {Promise<number>} - Current block number
 */
async function getCurrentBlockNumber(api) {
  const block = await api.rpc.chain.getBlock();
  return block.block.header.number.toNumber();
}

/**
 * Wait for a specific number of blocks to be produced
 * @param {ApiPromise} api - API instance
 * @param {number} blockCount - Number of blocks to wait for
 * @param {number} timeout - Timeout in milliseconds
 * @returns {Promise<number>} - Final block number
 */
async function waitForBlocks(api, blockCount, timeout = 60000) {
  const startBlock = await getCurrentBlockNumber(api);
  const targetBlock = startBlock + blockCount;
  
  log.info(`Waiting for ${blockCount} blocks (from #${startBlock} to #${targetBlock})...`);
  
  return new Promise((resolve, reject) => {
    const timeoutId = setTimeout(() => {
      reject(new Error(`Timeout waiting for blocks. Current: ${currentBlock}, Target: ${targetBlock}`));
    }, timeout);
    
    let currentBlock = startBlock;
    
    const unsubscribe = api.rpc.chain.subscribeNewHeads((header) => {
      currentBlock = header.number.toNumber();
      
      if (currentBlock >= targetBlock) {
        clearTimeout(timeoutId);
        unsubscribe.then(() => {
          log.success(`Reached block #${currentBlock}`);
          resolve(currentBlock);
        });
      }
    });
  });
}

/**
 * Get account balance
 * @param {ApiPromise} api - API instance
 * @param {string} address - Account address
 * @returns {Promise<{free: string, reserved: string, frozen: string}>} - Balance information
 */
async function getAccountBalance(api, address) {
  const { data: balance } = await api.query.system.account(address);
  return {
    free: balance.free.toString(),
    reserved: balance.reserved.toString(),
    frozen: balance.frozen?.toString() || balance.miscFrozen?.toString() || '0',
  };
}

/**
 * Get parachain ID
 * @param {ApiPromise} api - Parachain API instance
 * @returns {Promise<number>} - Parachain ID
 */
async function getParachainId(api) {
  const parachainId = await api.query.parachainInfo.parachainId();
  return parachainId.toNumber();
}

/**
 * Wait for an event to occur
 * @param {ApiPromise} api - API instance
 * @param {string} section - Event section (pallet name)
 * @param {string} method - Event method name
 * @param {number} timeout - Timeout in milliseconds
 * @returns {Promise<object>} - Event data
 */
async function waitForEvent(api, section, method, timeout = 60000) {
  log.info(`Waiting for event ${section}.${method}...`);
  
  return new Promise((resolve, reject) => {
    const timeoutId = setTimeout(() => {
      unsubscribe.then(() => {
        reject(new Error(`Timeout waiting for event ${section}.${method}`));
      });
    }, timeout);
    
    const unsubscribe = api.query.system.events((events) => {
      events.forEach((record) => {
        const { event } = record;
        
        if (event.section === section && event.method === method) {
          clearTimeout(timeoutId);
          unsubscribe.then(() => {
            log.success(`Event ${section}.${method} detected`);
            resolve(event.data);
          });
        }
      });
    });
  });
}

/**
 * Subscribe to events and filter by condition
 * @param {ApiPromise} api - API instance
 * @param {Function} filter - Filter function (event) => boolean
 * @param {number} timeout - Timeout in milliseconds
 * @returns {Promise<object[]>} - Matching events
 */
async function subscribeAndWaitForEvents(api, filter, timeout = 60000) {
  const matchingEvents = [];
  
  return new Promise((resolve, reject) => {
    const timeoutId = setTimeout(() => {
      unsubscribe.then(() => {
        if (matchingEvents.length > 0) {
          resolve(matchingEvents);
        } else {
          reject(new Error('Timeout waiting for matching events'));
        }
      });
    }, timeout);
    
    const unsubscribe = api.query.system.events((events) => {
      events.forEach((record) => {
        if (filter(record.event)) {
          matchingEvents.push(record.event);
        }
      });
    });
  });
}

/**
 * Get chain metadata
 * @param {ApiPromise} api - API instance
 * @returns {Promise<object>} - Chain metadata including name, version, etc.
 */
async function getChainMetadata(api) {
  const [chain, nodeName, nodeVersion, runtimeVersion] = await Promise.all([
    api.rpc.system.chain(),
    api.rpc.system.name(),
    api.rpc.system.version(),
    api.rpc.state.getRuntimeVersion(),
  ]);
  
  return {
    chain: chain.toString(),
    nodeName: nodeName.toString(),
    nodeVersion: nodeVersion.toString(),
    specName: runtimeVersion.specName.toString(),
    specVersion: runtimeVersion.specVersion.toNumber(),
    implVersion: runtimeVersion.implVersion.toNumber(),
  };
}

/**
 * Check if a pallet exists in the runtime
 * @param {ApiPromise} api - API instance
 * @param {string} palletName - Pallet name to check
 * @returns {boolean} - True if pallet exists
 */
function hasPallet(api, palletName) {
  return api.tx[palletName] !== undefined;
}

/**
 * Check if a specific extrinsic exists in a pallet
 * @param {ApiPromise} api - API instance
 * @param {string} palletName - Pallet name
 * @param {string} extrinsicName - Extrinsic name
 * @returns {boolean} - True if extrinsic exists
 */
function hasExtrinsic(api, palletName, extrinsicName) {
  return hasPallet(api, palletName) && api.tx[palletName][extrinsicName] !== undefined;
}

/**
 * Format balance for display
 * @param {string|number} balance - Balance in planck units
 * @param {number} decimals - Token decimals (default 12 for DOT/KSM)
 * @returns {string} - Formatted balance
 */
function formatBalance(balance, decimals = 12) {
  const balanceStr = balance.toString();
  const bigBalance = BigInt(balanceStr);
  const bigDivisor = 10n ** BigInt(decimals);
  const wholePart = bigBalance / bigDivisor;
  const remainder = bigBalance % bigDivisor;
  const remainderStr = remainder.toString().padStart(decimals, '0');
  return `${wholePart.toString()}.${remainderStr}`;
}

module.exports = {
  log,
  sleep,
  connectToNode,
  getCurrentBlockNumber,
  waitForBlocks,
  getAccountBalance,
  getParachainId,
  waitForEvent,
  subscribeAndWaitForEvents,
  getChainMetadata,
  hasPallet,
  hasExtrinsic,
  formatBalance,
};
