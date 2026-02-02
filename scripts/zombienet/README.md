# Robonomics Zombienet XCM Tests

Comprehensive integration tests for Robonomics parachain using [Zombienet](https://github.com/paritytech/zombienet), focusing on XCM (Cross-Consensus Messaging) functionality.

## Overview

This test suite validates the XCM integration of the Robonomics parachain, including:

- **Upward Messages (UMP):** Parachain → Relay Chain communication
- **Downward Messages (DMP):** Relay Chain → Parachain communication  
- **Relay Token Transfers:** Transferring relay chain tokens (KSM/DOT) to parachain
- **AssetHub Integration:** Cross-parachain asset transfers and wrapping

## Directory Structure

```
scripts/zombienet/
├── configs/                    # Network topology configurations
│   ├── robonomics-local.toml  # Basic local setup
│   ├── xcm-tests.toml         # Main XCM test configuration
│   ├── assethub-xcm.toml      # AssetHub-specific tests
│   └── README.md              # Configuration documentation
├── tests/                      # Test scripts
│   ├── integration-tests.js   # Main test runner
│   ├── xcm-tests.js           # XCM test implementations
│   ├── relay-token-transfer.test.js  # Relay token transfer tests
│   ├── package.json           # Node.js dependencies
│   └── helpers/               # Utility modules
│       ├── chain-utils.js     # Chain interaction utilities
│       └── xcm-utils.js       # XCM message construction utilities
├── tsconfig.json              # TypeScript configuration
├── run-tests.sh               # Test execution script
├── spawn-network.sh           # Network spawning script
├── ADDING_TESTS.md            # Guide for adding new tests
└── README.md                  # This file
```

## Prerequisites

### Required Software

1. **Zombienet CLI** (v1.3.0+)
   ```bash
   # Download from releases
   wget https://github.com/paritytech/zombienet/releases/download/v1.3.100/zombienet-linux-x64
   chmod +x zombienet-linux-x64
   sudo mv zombienet-linux-x64 /usr/local/bin/zombienet
   ```

2. **Polkadot Binary** (relay chain)
   ```bash
   # Download from releases
   wget https://github.com/paritytech/polkadot-sdk/releases/download/polkadot-v1.7.0/polkadot
   chmod +x polkadot
   sudo mv polkadot /usr/local/bin/
   ```

3. **Polkadot-parachain Binary** (for AssetHub)
   ```bash
   # Usually included with Polkadot releases
   wget https://github.com/paritytech/polkadot-sdk/releases/download/polkadot-v1.7.0/polkadot-parachain
   chmod +x polkadot-parachain
   sudo mv polkadot-parachain /usr/local/bin/
   ```

4. **Robonomics Binary** (build from source)
   ```bash
   cd /path/to/robonomics
   cargo build --release
   sudo cp target/release/robonomics /usr/local/bin/
   ```

5. **Node.js** (v18+) and **npm**
   ```bash
   # Install via nvm
   curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
   nvm install 18
   nvm use 18
   ```

### Verify Installation

```bash
zombienet version
polkadot --version
polkadot-parachain --version
robonomics --version
node --version
```

## Quick Start

### 1. Spawn Test Network

```bash
cd scripts/zombienet

# Spawn network with XCM test configuration
./spawn-network.sh

# Or specify a different configuration
./spawn-network.sh configs/assethub-xcm.toml
```

The network will start with:
- **Relay Chain:** 2 validators (Alice, Bob) on ports 9944-9945
- **AssetHub:** 1 collator on port 9910
- **Robonomics:** 2 collators on ports 9988-9989

### 2. Run Tests

In a separate terminal:

```bash
cd scripts/zombienet

# Run all tests
./run-tests.sh

# Or run tests directly
cd tests
npm install
npm test
```

### 3. Monitor Logs

Zombienet creates a log directory with outputs from all nodes:

```bash
# Check relay chain logs
tail -f /tmp/zombie-*/alice.log

# Check parachain logs
tail -f /tmp/zombie-*/robonomics-collator-01.log

# Watch XCM messages
grep -i "xcm" /tmp/zombie-*/*.log
```

## Test Categories

### 1. UMP Tests (Upward Messages)

**File:** `tests/xcm-tests.js` → `testXcmUpwardMessage()`

**What it tests:**
- Sending XCM messages from Robonomics parachain to relay chain
- `cumulus_primitives_utility::ParentAsUmp` router
- Message execution with `SovereignAccount` origin
- Event emission on relay chain

**Expected behavior:**
- Transaction is accepted on parachain
- `polkadotXcm.Sent` event is emitted
- Message appears in relay chain's UMP queue
- Message is processed on relay chain

### 2. DMP Tests (Downward Messages)

**File:** `tests/xcm-tests.js` → `testXcmDownwardMessage()`

**What it tests:**
- Sending XCM messages from relay chain to Robonomics parachain
- DMP queue processing via `cumulus_pallet_dmp_queue`
- Sudo-based message sending (relay chain privilege)
- Message execution with `Superuser` origin

**Expected behavior:**
- Sudo call succeeds on relay chain
- Message is queued in DMP
- Parachain receives and processes message
- `dmpQueue.Processed` or similar event on parachain

### 3. Relay Token Transfer Tests

**File:** `tests/relay-token-transfer.test.js`

**What it tests:**
- Reserve-backed token transfers from relay chain
- `AssetsFrom<RelayLocation>` reserve filter
- Sovereign account balance handling
- `XcmReserveTransferFilter` configuration
- Balance updates on both chains

**Expected behavior:**
- Tokens are deducted from sender on relay chain
- Sovereign account receives tokens on relay chain
- Equivalent tokens are minted/credited on parachain
- Both `reserveTransferAssets` and `limitedReserveTransferAssets` work

### 4. AssetHub Integration Tests

**File:** `tests/xcm-tests.js` → `testAssetHubTransfer()`

**What it tests:**
- XCMP communication between Robonomics and AssetHub
- Cross-parachain asset transfers
- `pallet-assets` integration
- Asset wrapping functionality (if applicable)

**Expected behavior:**
- Assets are transferred via XCMP
- `xcmpQueue` handles message routing
- Assets are received on destination parachain
- Balances are updated correctly

## Configuration Details

### Network Topology

All configurations use:
- **Relay Chain:** Rococo (local testnet variant)
- **Robonomics Parachain ID:** 2048 (production ID, as per `ROBONOMICS_PARA_ID`)
  - Note: Some local configs may use 2000 for simplicity
- **AssetHub Parachain ID:** 1000 (standard AssetHub ID)

### XCM Configuration Tested

Tests validate the runtime configuration in `runtime/robonomics/src/xcm_config.rs`:

```rust
// XCM Router: UMP + XCMP
pub type XcmRouter = (
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>,
    XcmpQueue,
);

// Reserve asset filter
pub type XcmConfig = {
    type IsReserve = AssetsFrom<RelayLocation>;
    type XcmReserveTransferFilter = Everything; // Or more restrictive
    // ...
};

// Asset transactors
pub type AssetTransactors = (CurrencyTransactor, FungiblesTransactor);
```

### Logging Levels

Tests run with enhanced logging:
- `-lparachain=debug`: Parachain consensus details
- `-lxcm=trace`: Full XCM message traces
- `-lassets=trace`: Asset operations (for AssetHub tests)

## Troubleshooting

### Network Fails to Start

**Symptom:** Zombienet times out or nodes crash immediately

**Solutions:**
1. Verify all binaries are in PATH: `which polkadot robonomics polkadot-parachain`
2. Check port availability: `lsof -i :9944 -i :9988 -i :9910`
3. Review zombienet logs in `/tmp/zombie-*`
4. Ensure you have enough disk space and memory (>4GB RAM recommended)

### Tests Timeout

**Symptom:** Tests hang or timeout waiting for events

**Solutions:**
1. Increase timeout values in test config (e.g., `TESTS_CONFIG.timeout`)
2. Check if network is producing blocks: Monitor logs or use Polkadot.js Apps
3. Verify XCM channels are open: Check `hrmpChannels` on relay chain
4. Review XCM execution logs: `grep -i "xcm" /tmp/zombie-*/*.log`

### XCM Messages Not Delivered

**Symptom:** Messages sent but never processed on destination

**Solutions:**
1. **Check HRMP channels:** XCM between parachains requires open HRMP channels
   ```bash
   # In Polkadot.js Apps connected to relay chain
   # Navigate to: Developer > Chain State > hrmp > hrmpChannels
   ```

2. **Verify weights:** XCM execution may fail if weights are too low
   - Check `UnitWeightCost` in `xcm_config.rs`
   - Look for `Overweight` events

3. **Check barriers:** XCM barriers may reject messages
   - Review `XcmBarrier` configuration
   - Ensure `AllowTopLevelPaidExecutionFrom` includes your origins

4. **Fee issues:** Messages may fail if fees are insufficient
   - Increase asset amounts in test
   - Check `BuyExecution` instruction

### Balance Not Updated After Transfer

**Symptom:** XCM transfer succeeds but balance doesn't change

**Solutions:**
1. **Check asset storage:** Relay tokens may be stored in `pallet-assets`, not `pallet-balances`
   ```javascript
   // Query pallet-assets instead
   const asset = await api.query.assets.account(assetId, address);
   ```

2. **Verify asset mapping:** Check `XcmInfo` pallet for asset ID mappings

3. **Sovereign account:** Tokens may be in parachain's sovereign account on relay chain
   ```javascript
   const sovereign = calculateSovereignAccount(parachainId);
   const balance = await relayApi.query.system.account(sovereign);
   ```

### TypeScript Compilation Errors

**Symptom:** `tsc` fails when building

**Solutions:**
1. Install dev dependencies: `npm install`
2. Update TypeScript: `npm install typescript@latest --save-dev`
3. Check tsconfig.json is valid JSON
4. For now, tests are JavaScript - TypeScript is optional

## Advanced Usage

### Running Individual Tests

```bash
cd tests

# Run only XCM tests
node -e "require('./xcm-tests').testXcmUpwardMessage({ total: 0, passed: 0, failed: 0 })"

# Run only relay token transfer
node -e "require('./relay-token-transfer.test').testRelayTokenTransfer({ total: 0, passed: 0, failed: 0 })"
```

### Custom Network Configuration

1. Create a new TOML file in `configs/`:
   ```toml
   [relaychain]
   chain = "rococo-local"
   # ... your configuration
   ```

2. Spawn with your config:
   ```bash
   ./spawn-network.sh configs/my-config.toml
   ```

3. Update test configs if using different ports

### Adding New Tests

See [ADDING_TESTS.md](./ADDING_TESTS.md) for a comprehensive guide on extending the test suite.

Key steps:
1. Create test function following the pattern
2. Import in `integration-tests.js`
3. Add to `runTests()` function
4. Run and verify

## Test Helper Utilities

### Chain Utilities (`helpers/chain-utils.js`)

```javascript
const { connectToNode, getAccountBalance, waitForBlocks } = require('./helpers/chain-utils');

// Connect with retry logic
const api = await connectToNode('ws://127.0.0.1:9988', 'Parachain');

// Get account balance
const balance = await getAccountBalance(api, address);

// Wait for block production
await waitForBlocks(api, 5); // Wait for 5 blocks
```

### XCM Utilities (`helpers/xcm-utils.js`)

```javascript
const { 
  createParachainDestination,
  createBeneficiary,
  createNativeAsset,
  toPlanck,
} = require('./helpers/xcm-utils');

// Create XCM message components
const dest = createParachainDestination(1000); // AssetHub
const beneficiary = createBeneficiary(alice.publicKey);
const assets = createNativeAsset(toPlanck(1, 12)); // 1 token

// Calculate sovereign account
const sovereign = calculateSovereignAccount(2048);
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Zombienet XCM Tests

on:
  pull_request:
    branches: [main, release/*]

jobs:
  xcm-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install dependencies
        run: |
          # Install zombienet, polkadot, etc.
          
      - name: Build Robonomics
        run: cargo build --release
      
      - name: Run XCM tests
        run: |
          cd scripts/zombienet
          ./spawn-network.sh configs/xcm-tests.toml &
          sleep 60  # Wait for network
          ./run-tests.sh
```

## Runtime Configuration Reference

Tests validate these key runtime components:

### `pallet_xcm::Config`
- `XcmRouter`: UMP and XCMP routing
- `XcmReserveTransferFilter`: Reserve transfer permissions
- `XcmTeleportFilter`: Teleport permissions (currently `Nothing`)
- `Weigher`: Weight calculation for XCM execution

### `cumulus_pallet_xcmp_queue::Config`
- XCMP message queue management
- Cross-parachain message delivery
- Exponential pricing for sibling delivery

### `cumulus_pallet_dmp_queue::Config` (if applicable)
- Downward message processing
- Message weight limits
- Queue management

### Asset Configuration
- `AssetsFrom<RelayLocation>`: Accept assets from relay chain
- `FungiblesTransactor`: Handle `pallet-assets` via XCM
- `CurrencyTransactor`: Handle native currency via XCM

## Further Reading

- [Zombienet Documentation](https://paritytech.github.io/zombienet/)
- [XCM Format Specification](https://wiki.polkadot.network/docs/learn-xcm)
- [Cumulus Parachain Tutorial](https://docs.substrate.io/tutorials/connect-other-chains/local-relay/)
- [Robonomics XCM Config](../../runtime/robonomics/src/xcm_config.rs)
- [XCM v5 Migration Guide](https://github.com/paritytech/polkadot-sdk/blob/master/docs/XCM_v5_MIGRATION.md)

## Contributing

When adding tests:
1. Follow existing code patterns
2. Add comprehensive logging
3. Handle errors gracefully
4. Update documentation
5. Test locally before submitting PR

## License

Apache 2.0 - see [LICENSE](../../LICENSE)

## Support

For issues or questions:
- [Robonomics GitHub Issues](https://github.com/airalab/robonomics/issues)
- [Robonomics Discord](https://discord.gg/robonomics)
- [Developer Documentation](https://wiki.robonomics.network/)
