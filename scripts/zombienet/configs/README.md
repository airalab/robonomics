# Zombienet Configuration Files

This directory contains Zombienet network topology configurations for testing the Robonomics parachain with various XCM scenarios.

## Configuration Files

### robonomics-local.toml
Basic local network configuration with:
- Relay chain (Rococo): 2 validators (Alice, Bob)
- AssetHub parachain (ID 1000): 1 collator
- Robonomics parachain (ID 2000): 2 collators

**Use case:** Basic local development and testing without XCM focus.

**WebSocket endpoints:**
- Relay chain: `ws://127.0.0.1:9944`
- AssetHub: `ws://127.0.0.1:9910`
- Robonomics: `ws://127.0.0.1:9988`

### xcm-tests.toml
Comprehensive XCM testing configuration with:
- Relay chain (Rococo): 2 validators with XCM tracing enabled
- AssetHub parachain (ID 1000): 1 collator with XCM tracing
- Robonomics parachain (ID 2048): 2 collators with XCM tracing

**Use case:** Testing UMP, DMP, XCMP messages, asset transfers, and AssetHub integration.

**Features:**
- Enhanced logging with `-lxcm=trace` for debugging XCM messages
- Proper parachain ID (2048) matching production configuration
- Support for cross-parachain messaging (XCMP)
- Comprehensive coverage for all XCM test scenarios

**WebSocket endpoints:**
- Relay chain: `ws://127.0.0.1:9944`
- AssetHub: `ws://127.0.0.1:9910`
- Robonomics: `ws://127.0.0.1:9988` (collator 1), `ws://127.0.0.1:9989` (collator 2)

## Parachain IDs

- **AssetHub:** 1000 (standard Kusama/Polkadot AssetHub ID)
- **Robonomics:** 2048 (as defined in `runtime/robonomics/src/genesis_config_presets.rs`)

Note: In local development configs, Robonomics may use ID 2000 for convenience, but production uses 2048.

## Runtime Configuration

All configurations test the XCM setup defined in `runtime/robonomics/src/xcm_config.rs`:

- **XcmRouter:** Uses UMP (Upward Message Passing) and XCMP (Cross-Chain Message Passing)
- **AssetTransactors:** Native currency and fungible assets
- **LocationToAccountId:** Handles relay chain, sibling parachains, and account conversion
- **XcmReserveTransferFilter:** Allows reserve transfers (currently set to `Everything`)
- **IsReserve:** `AssetsFrom<RelayLocation>` - accepts reserve assets from relay chain

## Running Configurations

### Using zombienet CLI:

```bash
# Basic local network (for simple development)
zombienet spawn configs/robonomics-local.toml -p native

# XCM test network (recommended for XCM testing)
zombienet spawn configs/xcm-tests.toml -p native
```

### Using the spawn script:

Update `spawn-network.sh` to use the desired configuration:

```bash
#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
zombienet spawn ${SCRIPT_DIR}/configs/xcm-tests.toml -p native
```

Then run:
```bash
./spawn-network.sh
```

## Prerequisites

1. **Zombienet CLI:** Install from [zombienet releases](https://github.com/paritytech/zombienet/releases)
2. **Polkadot binary:** Download or build from [polkadot repo](https://github.com/paritytech/polkadot)
3. **Polkadot-parachain binary:** For AssetHub (included in Polkadot releases)
4. **Robonomics binary:** Build from this repository:
   ```bash
   cargo build --release
   ```

Ensure all binaries are in your `$PATH` or update the `command` fields in the TOML files to point to the binary locations.

## Troubleshooting

### Port conflicts
If ports are already in use, modify the `rpc_port` and `ws_port` values in the configuration files.

### Connection timeouts
Increase the `timeout` value in the `[settings]` section if the network takes longer to initialize.

### Binary not found
Ensure all required binaries (`polkadot`, `polkadot-parachain`, `robonomics`) are in your PATH or specify full paths in the config.

### Parachain not producing blocks
- Check that the relay chain has at least 2 validators running
- Verify collators are connected to the relay chain
- Check logs for HRMP channel opening issues

### XCM messages not being delivered
- Verify XCM channels are open between chains
- Check weight and fee configuration in runtime
- Review logs with `-lxcm=trace` for detailed XCM execution info

## Further Reading

- [Zombienet Documentation](https://paritytech.github.io/zombienet/)
- [XCM Format](https://wiki.polkadot.network/docs/learn-xcm)
- [Cumulus Tutorial](https://docs.substrate.io/tutorials/connect-other-chains/local-relay/)
- [Robonomics Runtime Configuration](../../runtime/robonomics/src/xcm_config.rs)
