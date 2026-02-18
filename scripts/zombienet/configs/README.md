# Zombienet Configuration Files

This directory contains Zombienet network topology configurations for testing the Robonomics parachain with XCM scenarios.

## Configuration Files

### robonomics-local.toml
Local network configuration with XCM tracing enabled:
- Relay chain (Rococo): 2 validators (Alice, Bob)
- AssetHub parachain (ID 1000): 1 collator
- Robonomics parachain (ID 2000): 2 collators
- Enhanced logging with `-lxcm=trace` for debugging XCM messages

**Use case:** Local development and comprehensive XCM testing (UMP, DMP, XCMP messages, asset transfers, and AssetHub integration).

**WebSocket endpoints:**
- Relay chain: `ws://127.0.0.1:9944`
- AssetHub: `ws://127.0.0.1:9910`
- Robonomics: `ws://127.0.0.1:9988`, `ws://127.0.0.1:9989`

## Parachain IDs

- **AssetHub:** 1000 (standard Kusama/Polkadot AssetHub ID)
- **Robonomics:** 2000 (local development) / 2048 (production, as defined in `runtime/robonomics/src/genesis_config_presets.rs`)

Note: Local configuration uses ID 2000 for convenience, but production uses 2048.

## Runtime Configuration

All configurations test the XCM setup defined in `runtime/robonomics/src/xcm_config.rs`:

- **XcmRouter:** Uses UMP (Upward Message Passing) and XCMP (Cross-Chain Message Passing)
- **AssetTransactors:** Native currency and fungible assets
- **LocationToAccountId:** Handles relay chain, sibling parachains, and account conversion
- **XcmReserveTransferFilter:** Allows reserve transfers (currently set to `Everything`)
- **IsReserve:** `AssetsFrom<RelayLocation>` - accepts reserve assets from relay chain

## Running Configuration

### Using zombienet CLI:

```bash
zombienet spawn configs/robonomics-local.toml -p native
```

### Using the spawn script:

```bash
./spawn-network.sh
```

## Prerequisites

All required binaries (polkadot, polkadot-parachain, zombienet, robonomics) are available in the `local-testnet` nix devshell. See the main README for setup instructions.

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
