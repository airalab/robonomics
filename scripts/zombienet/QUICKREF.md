# Zombienet Integration Tests - Quick Reference

## Quick Commands

```bash
# Run all tests with automatic setup
./scripts/zombienet/run-tests.sh

# Run tests skipping setup (if already configured)
./scripts/zombienet/run-tests.sh --skip-setup

# Show help
./scripts/zombienet/run-tests.sh --help

# Manually spawn network (for debugging)
cd scripts/zombienet
./bin/zombienet spawn robonomics-local.toml --provider native
```

## Test Coverage

| Test Name | Purpose | Duration |
|-----------|---------|----------|
| Network Initialization | Verifies relay chain and parachain nodes are accessible | ~30s |
| Block Production | Confirms blocks are being produced on both chains | ~1min |
| Extrinsic Submission | Tests transaction submission and inclusion | ~1min |

## Configuration Files

- `robonomics-local.toml` - Zombienet network configuration
- `tests/integration-tests.js` - Test suite implementation
- `tests/package.json` - Node.js dependencies
- `run-tests.sh` - Main test runner script

## Default Ports

| Component | WebSocket | P2P |
|-----------|-----------|-----|
| Relay Chain (Alice) | 9944 | 30333 |
| Relay Chain (Bob) | 9945 | 30334 |
| Parachain (Collator 01) | 9988 | 31200 |
| Parachain (Collator 02) | 9989 | 31201 |

## Environment Variables

Currently, the tests use default configurations. To customize:

- Edit `TESTS_CONFIG` in `tests/integration-tests.js`
- Modify network settings in `robonomics-local.toml`

## Common Test Scenarios

### Testing a Specific Feature

1. Spawn the network manually:
   ```bash
   cd scripts/zombienet
   ./bin/zombienet spawn robonomics-local.toml --provider native
   ```

2. In another terminal, connect with polkadot.js:
   ```bash
   # Relay chain: ws://127.0.0.1:9944
   # Parachain: ws://127.0.0.1:9988
   ```

3. Run custom tests or interact manually

### Debugging Failed Tests

1. Check zombienet logs (printed during test execution)
2. Inspect network state at the printed WebSocket URLs
3. Review test output for specific error messages
4. Increase timeout values if needed

## CI/CD Integration

The tests are configured to run automatically via GitHub Actions:
- On pull requests to `master` and `release/*` branches
- On pushes to `master`
- Can be triggered manually via workflow_dispatch

View workflow: `.github/workflows/zombienet.yml`

## Dependencies

### Runtime
- Zombienet binary (auto-downloaded)
- Polkadot binary (auto-downloaded)
- Robonomics binary (must be built: `cargo build --release`)

### Node.js Packages
- `@polkadot/api` - Polkadot.js API
- `@polkadot/keyring` - Key management
- `@polkadot/util` - Utility functions
- `@polkadot/util-crypto` - Cryptographic utilities

## Best Practices

1. **Always build Robonomics first** before running tests
2. **Clean ports** if tests fail to start (check for zombie processes)
3. **Run locally** before pushing to CI to save resources
4. **Check logs** in `/tmp/zombie-*` directories for detailed output
5. **Update timeouts** if network is slow or heavily loaded

## Troubleshooting Matrix

| Issue | Cause | Solution |
|-------|-------|----------|
| Port in use | Previous test didn't clean up | Kill processes: `pkill -f zombienet` |
| Binary not found | Robonomics not built | Run: `cargo build --release` |
| Connection timeout | Network starting slowly | Increase wait times in config |
| Transaction fails | Insufficient gas/funds | Check Alice account balance |
| Node crashes | Out of memory | Close other applications, check RAM |

## Performance Tips

- **Disk I/O**: Use SSD for faster block production
- **CPU**: Tests run best on multi-core systems
- **Memory**: Close unnecessary applications for smoother tests
- **Network**: Local tests don't need internet (except initial downloads)

## Future Enhancements

Potential additions to the test suite:
- XCM message passing tests
- Parachain upgrade tests
- Validator/collator rotation tests
- RWS (Robonomics Web Services) specific tests
- DataLog and Launch pallet tests
- Digital Twin functionality tests

## Support Resources

- [Zombienet GitHub](https://github.com/paritytech/zombienet)
- [Polkadot.js Docs](https://polkadot.js.org/docs/)
- [Robonomics Wiki](https://wiki.robonomics.network)
- [Project Issues](https://github.com/airalab/robonomics/issues)
