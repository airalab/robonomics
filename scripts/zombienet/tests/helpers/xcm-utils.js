/**
 * XCM Utilities for Zombienet Tests
 * 
 * Helper functions for constructing and testing XCM messages.
 */

const { Keyring } = require('@polkadot/keyring');
const { u8aToHex } = require('@polkadot/util');

/**
 * Create a versioned XCM location
 * @param {number} parents - Number of parent chains
 * @param {object|string} interior - Interior location (can be 'Here' or junction array)
 * @param {number} version - XCM version (default 5)
 * @returns {object} - Versioned location
 */
function createLocation(parents, interior = 'Here', version = 5) {
  const versionKey = `V${version}`;
  return {
    [versionKey]: {
      parents,
      interior,
    },
  };
}

/**
 * Create a parachain junction
 * @param {number|string} parachainId - Parachain ID
 * @returns {object} - Parachain junction
 */
function createParachainJunction(parachainId) {
  return { Parachain: parachainId.toString() };
}

/**
 * Create an AccountId32 junction
 * @param {Uint8Array|string} accountId - Account ID (32 bytes)
 * @param {string|null} network - Network ID (null for any network)
 * @returns {object} - AccountId32 junction
 */
function createAccountId32Junction(accountId, network = null) {
  return {
    AccountId32: {
      network,
      id: accountId,
    },
  };
}

/**
 * Create a destination location for a parachain
 * @param {number} parachainId - Target parachain ID
 * @param {number} version - XCM version (default 5)
 * @returns {object} - Destination location
 */
function createParachainDestination(parachainId, version = 5) {
  return createLocation(1, { X1: [createParachainJunction(parachainId)] }, version);
}

/**
 * Create a beneficiary location for an account on a parachain
 * @param {Uint8Array|string} accountId - Beneficiary account ID
 * @param {string|null} network - Network ID
 * @param {number} version - XCM version (default 5)
 * @returns {object} - Beneficiary location
 */
function createBeneficiary(accountId, network = null, version = 5) {
  return createLocation(0, { X1: [createAccountId32Junction(accountId, network)] }, version);
}

/**
 * Create a fungible asset
 * @param {number} parents - Parents in the asset location
 * @param {object|string} interior - Interior of the asset location
 * @param {string|number} amount - Amount of the asset (in planck units)
 * @param {number} version - XCM version (default 5)
 * @returns {object} - Asset definition
 */
function createAsset(parents, interior, amount, version = 5) {
  const versionKey = `V${version}`;
  return {
    [versionKey]: [
      {
        id: { parents, interior },
        fun: { Fungible: amount.toString() },
      },
    ],
  };
}

/**
 * Create a native token asset (local chain token)
 * @param {string|number} amount - Amount in planck units
 * @param {number} version - XCM version (default 5)
 * @returns {object} - Native asset
 */
function createNativeAsset(amount, version = 5) {
  return createAsset(0, 'Here', amount, version);
}

/**
 * Create a relay chain token asset
 * @param {string|number} amount - Amount in planck units
 * @param {number} version - XCM version (default 5)
 * @returns {object} - Relay chain asset
 */
function createRelayAsset(amount, version = 5) {
  return createAsset(1, 'Here', amount, version);
}

/**
 * Create an XCM message with Transact instruction
 * @param {string} encodedCall - Hex-encoded call data
 * @param {string} originKind - Origin kind (e.g., 'SovereignAccount', 'Superuser')
 * @param {object} weightLimit - Weight limit object (e.g., 'Unlimited' or { Limited: value })
 * @param {number} version - XCM version (default 5)
 * @returns {object} - XCM message
 */
function createTransactMessage(encodedCall, originKind = 'SovereignAccount', weightLimit = 'Unlimited', version = 5) {
  const versionKey = `V${version}`;
  return {
    [versionKey]: [
      {
        UnpaidExecution: {
          weight_limit: weightLimit,
        },
      },
      {
        Transact: {
          origin_kind: originKind,
          call: {
            encoded: encodedCall,
          },
        },
      },
    ],
  };
}

/**
 * Create an XCM message with WithdrawAsset and BuyExecution
 * @param {object} asset - Asset to withdraw
 * @param {string} encodedCall - Hex-encoded call data
 * @param {string} originKind - Origin kind
 * @param {number} version - XCM version (default 5)
 * @returns {object} - XCM message
 */
function createWithdrawAndTransactMessage(asset, encodedCall, originKind = 'SovereignAccount', version = 5) {
  const versionKey = `V${version}`;
  const assetData = asset[versionKey][0];
  
  return {
    [versionKey]: [
      {
        WithdrawAsset: [assetData],
      },
      {
        BuyExecution: {
          fees: assetData,
          weight_limit: 'Unlimited',
        },
      },
      {
        Transact: {
          origin_kind: originKind,
          call: {
            encoded: encodedCall,
          },
        },
      },
    ],
  };
}

/**
 * Create a keyring with standard test accounts
 * @param {string} type - Key type (default 'sr25519')
 * @returns {object} - Object with Alice, Bob, Charlie, etc.
 */
function createTestKeyring(type = 'sr25519') {
  const keyring = new Keyring({ type });
  
  return {
    alice: keyring.addFromUri('//Alice'),
    bob: keyring.addFromUri('//Bob'),
    charlie: keyring.addFromUri('//Charlie'),
    dave: keyring.addFromUri('//Dave'),
    eve: keyring.addFromUri('//Eve'),
    ferdie: keyring.addFromUri('//Ferdie'),
  };
}

/**
 * Calculate sovereign account address for a parachain
 * @param {number} parachainId - Parachain ID
 * @returns {string} - Sovereign account address
 */
function calculateSovereignAccount(parachainId) {
  const { encodeAddress } = require('@polkadot/util-crypto');
  const { blake2AsU8a } = require('@polkadot/util-crypto');
  
  // Create the prefix "para" in bytes
  const prefix = new TextEncoder().encode('para');
  
  // Encode parachain ID as 4-byte little-endian
  const idBytes = new Uint8Array(4);
  const view = new DataView(idBytes.buffer);
  view.setUint32(0, parachainId, true); // little-endian
  
  // Concatenate and hash
  const combined = new Uint8Array(prefix.length + idBytes.length);
  combined.set(prefix);
  combined.set(idBytes, prefix.length);
  
  const hashed = blake2AsU8a(combined, 256);
  
  // Return SS58 encoded address
  return encodeAddress(hashed);
}

/**
 * Check if an XCM event indicates success
 * @param {object} event - Event object
 * @returns {boolean} - True if event indicates XCM success
 */
function isXcmSuccessEvent(event) {
  const successSections = ['polkadotXcm', 'xcmPallet', 'messageQueue', 'dmpQueue', 'xcmpQueue'];
  const successMethods = ['Sent', 'Success', 'Processed', 'ExecutedDownward'];
  
  return successSections.includes(event.section) && successMethods.includes(event.method);
}

/**
 * Check if an XCM event indicates failure
 * @param {object} event - Event object
 * @returns {boolean} - True if event indicates XCM failure
 */
function isXcmFailureEvent(event) {
  const failureSections = ['polkadotXcm', 'xcmPallet', 'messageQueue', 'dmpQueue', 'xcmpQueue'];
  const failureMethods = ['Fail', 'Failed', 'ProcessingFailed', 'ExecutionFailed', 'FailedToSend'];
  
  return failureSections.includes(event.section) && failureMethods.includes(event.method);
}

/**
 * Filter XCM-related events from a list of events
 * @param {Array} events - Array of event records
 * @returns {Array} - Filtered XCM events
 */
function filterXcmEvents(events) {
  return events.filter((record) => {
    const { event } = record;
    const xcmSections = ['polkadotXcm', 'xcmPallet', 'messageQueue', 'dmpQueue', 'xcmpQueue', 'ump'];
    return xcmSections.includes(event.section);
  });
}

/**
 * Wait for XCM execution with event monitoring
 * @param {ApiPromise} api - API instance
 * @param {number} timeout - Timeout in milliseconds
 * @param {Function} eventFilter - Optional custom event filter
 * @returns {Promise<object>} - Execution result with events
 */
async function waitForXcmExecution(api, timeout = 60000, eventFilter = null) {
  return new Promise((resolve, reject) => {
    const xcmEvents = [];
    let hasSuccess = false;
    let hasFailure = false;
    
    const timeoutId = setTimeout(() => {
      unsubscribe.then(() => {
        resolve({
          success: hasSuccess,
          failure: hasFailure,
          events: xcmEvents,
          timedOut: true,
        });
      });
    }, timeout);
    
    const unsubscribe = api.query.system.events((events) => {
      events.forEach((record) => {
        const { event } = record;
        
        // Apply custom filter if provided
        if (eventFilter && !eventFilter(event)) {
          return;
        }
        
        // Check for XCM events
        if (isXcmSuccessEvent(event)) {
          xcmEvents.push(event);
          hasSuccess = true;
        } else if (isXcmFailureEvent(event)) {
          xcmEvents.push(event);
          hasFailure = true;
        }
      });
      
      // Resolve early if we have a definitive result
      if (hasSuccess || hasFailure) {
        clearTimeout(timeoutId);
        unsubscribe.then(() => {
          resolve({
            success: hasSuccess,
            failure: hasFailure,
            events: xcmEvents,
            timedOut: false,
          });
        });
      }
    });
  });
}

/**
 * Convert token amount to planck units
 * @param {number} amount - Amount in tokens
 * @param {number} decimals - Token decimals (default 12)
 * @returns {string} - Amount in planck units
 */
function toPlanck(amount, decimals = 12) {
  // Use BigInt to avoid precision loss
  const bigAmount = BigInt(Math.floor(amount * Math.pow(10, decimals)));
  return bigAmount.toString();
}

/**
 * Convert planck units to token amount
 * @param {string|number} planck - Amount in planck units
 * @param {number} decimals - Token decimals (default 12)
 * @returns {number} - Amount in tokens
 */
function fromPlanck(planck, decimals = 12) {
  // Use BigInt to preserve precision
  const bigPlanck = BigInt(planck.toString());
  const divisor = BigInt(10 ** decimals);
  return Number(bigPlanck / divisor);
}

module.exports = {
  createLocation,
  createParachainJunction,
  createAccountId32Junction,
  createParachainDestination,
  createBeneficiary,
  createAsset,
  createNativeAsset,
  createRelayAsset,
  createTransactMessage,
  createWithdrawAndTransactMessage,
  createTestKeyring,
  calculateSovereignAccount,
  isXcmSuccessEvent,
  isXcmFailureEvent,
  filterXcmEvents,
  waitForXcmExecution,
  toPlanck,
  fromPlanck,
};
