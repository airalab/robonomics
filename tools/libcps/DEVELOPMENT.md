# Development Guide

This document provides information for developers working on the CPS CLI tool.

## Prerequisites

- Rust 1.88.0 or later
- A running Robonomics node with CPS pallet
- (Optional) MQTT broker for testing bridge functionality

## Building

```bash
# Build in debug mode
cargo build --package libcps

# Build in release mode (optimized)
cargo build --release --package libcps

# Build CLI binary only
cargo build --bin cps

# Build library only
cargo build --lib

# The binaries will be at:
# Debug: target/debug/cps
# Release: target/release/cps
```

### Feature Flags

The project supports several feature flags for flexible dependency management:

```bash
# Build with all features (default)
cargo build --package libcps

# Build library without MQTT support
cargo build --package libcps --lib --no-default-features

# Build library with only MQTT (no CLI)
cargo build --package libcps --lib --no-default-features --features mqtt

# Build CLI (includes MQTT by default)
cargo build --package libcps --bin cps
```

Available features:
- **`mqtt`** - Enables MQTT bridge functionality (default: enabled)
- **`cli`** - Enables CLI binary with colored output and chrono (default: enabled)

Feature dependencies:
- `default = ["mqtt", "cli"]`
- `mqtt = ["dep:rumqttc"]`
- `cli = ["mqtt", "clap", "colored", "chrono"]`

## Running

```bash
# Run with cargo
cargo run --package robonomics-cps-cli -- --help

# Or run the binary directly
./target/debug/cps --help
```

## Code Structure

### Main Components

1. **`src/lib.rs`**: Library entry point with module exports
2. **`src/main.rs`**: CLI entry point using `clap` for argument parsing
3. **`src/blockchain/`**: Blockchain client and connection management
4. **`src/commands/`**: Individual CLI command implementations (thin wrappers)
5. **`src/crypto/`**: Encryption/decryption utilities (library)
6. **`src/display/`**: Beautiful colored output formatting (CLI-only)
7. **`src/mqtt/`**: MQTT bridge configuration and implementation (library)
8. **`src/node.rs`**: Node-oriented API for CPS operations (library)
9. **`src/types.rs`**: Type definitions matching the CPS pallet (library)

### Adding a New Command

1. Create a new file in `src/commands/` (e.g., `my_command.rs`)
2. Implement the `execute` function
3. Add the module to `src/commands/mod.rs`
4. Add the command variant to the `Commands` enum in `src/main.rs`
5. Add the command handler in the match statement in `main()`

Example:

```rust
// src/commands/my_command.rs
use crate::blockchain::{Client, Config};
use crate::display;
use anyhow::Result;

pub async fn execute(config: &Config, param: String) -> Result<()> {
    display::tree::progress("Executing my command...");
    let client = Client::new(config).await?;
    // Your implementation here
    display::tree::success("Command completed!");
    Ok(())
}
```

## Blockchain Metadata

libcps automatically generates type definitions from the robonomics runtime during build.

### How It Works

The robonomics runtime is added as a build dependency. When libcps builds:

1. The runtime is compiled (if not already built)
2. The build script accesses the embedded WASM binary from the runtime
3. Metadata is automatically extracted from the WASM
4. The subxt macro uses this metadata to generate type-safe APIs

No manual steps or external tools required!

### Generated API

The generated API is available as `libcps::robonomics_api`:

```rust
use libcps::robonomics_api;

// Access runtime APIs
let create_call = robonomics_api::tx().cps().create_node(...);
let nodes_query = robonomics_api::storage().cps().nodes(node_id);
```

### Requirements

Just Rust and Cargo! The build process is fully automated:
- Runtime WASM is embedded via dependency
- Metadata extraction happens in build.rs
- No external tools needed

This ensures:
- Metadata is always in sync with the runtime
- Zero manual steps
- Type definitions are generated automatically
- Self-contained build process

## Testing

### Unit Tests

```bash
cargo test --package robonomics-cps-cli
```

### Integration Testing

To test with a live node:

1. Start a Robonomics development node:
   ```bash
   robonomics --dev --tmp
   ```

2. Set up environment:
   ```bash
   export ROBONOMICS_WS_URL=ws://localhost:9944
   export ROBONOMICS_SURI=//Alice
   ```

3. Run commands:
   ```bash
   cargo run --package robonomics-cps-cli -- create --meta '{"test":true}'
   cargo run --package robonomics-cps-cli -- show 0
   ```

### MQTT Testing

1. Start mosquitto broker:
   ```bash
   mosquitto -v
   ```

2. Test subscribe in one terminal:
   ```bash
   cargo run --package robonomics-cps-cli -- mqtt subscribe "test/topic" 0
   ```

3. Publish messages in another:
   ```bash
   mosquitto_pub -t "test/topic" -m "test message"
   ```

## Code Quality

### Linting

```bash
# Run clippy
cargo clippy --package robonomics-cps-cli

# Apply automatic fixes
cargo clippy --fix --package robonomics-cps-cli --allow-dirty
```

### Formatting

```bash
# Check formatting
cargo fmt --package robonomics-cps-cli -- --check

# Apply formatting
cargo fmt --package robonomics-cps-cli
```

## Debugging

### Enable Rust logging

```bash
export RUST_LOG=debug
cargo run --package robonomics-cps-cli -- show 0
```

### Using LLDB/GDB

```bash
# Build with debug symbols
cargo build --package robonomics-cps-cli

# Run with debugger
rust-lldb target/debug/cps
# or
rust-gdb target/debug/cps
```

## Dependencies

### Core Dependencies

- `subxt`: Substrate RPC client
- `subxt-signer`: Account signing utilities
- `clap`: Command-line argument parsing
- `tokio`: Async runtime
- `anyhow`: Error handling

### Crypto Dependencies

- `schnorrkel`: sr25519 cryptography
- `chacha20poly1305`: XChaCha20-Poly1305 encryption
- `hkdf`: HMAC-based key derivation
- `sha2`: SHA-256 hashing

### UI Dependencies

- `colored`: Terminal colors and formatting
- `serde`/`serde_json`: Serialization

### Optional Dependencies

- `rumqttc`: MQTT client

## Architecture Decisions

### Why XChaCha20-Poly1305?

- Large nonce space (192 bits) prevents nonce reuse
- Fast in software (no hardware requirements)
- Authenticated encryption (AEAD)
- Well-tested and widely adopted

### Why Subxt?

- Type-safe blockchain interactions
- Auto-generated types from metadata
- Async/await support
- Active development and good documentation

### Why Separate Commands?

- Modularity: Each command is self-contained
- Testability: Easy to test individual commands
- Maintainability: Clear code organization
- Extensibility: Easy to add new commands

## Common Issues

### Connection Refused

**Problem**: `Error when opening the TCP socket: Connection refused`

**Solution**: Make sure your Robonomics node is running:
```bash
robonomics --dev --tmp
```

### Missing Metadata

**Problem**: Types not found or compilation errors after generating metadata

**Solution**: Regenerate metadata with the correct node version:
```bash
subxt metadata --url ws://localhost:9944 > metadata.scale
subxt codegen --file metadata.scale > src/robonomics_runtime.rs
```

### Type Mismatch

**Problem**: SCALE encoding/decoding errors

**Solution**: Ensure your types match the pallet types exactly. Check:
- Field order
- Type names
- Encoding attributes (`#[codec(...)]`)

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and linting
5. Submit a pull request

### Code Style

- Follow Rust standard style (use `cargo fmt`)
- Add documentation comments for public functions
- Use descriptive variable names
- Keep functions focused and small
- Add tests for new functionality

### Commit Messages

Format: `<type>: <description>`

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes
- `refactor`: Code refactoring
- `test`: Test additions/changes
- `chore`: Maintenance tasks

Example:
```
feat: add support for encrypted MQTT messages
fix: handle disconnection in MQTT bridge
docs: update README with new examples
```

## Performance Considerations

### Blockchain Queries

- Use `at_latest()` for most recent state
- Consider caching frequently accessed data
- Batch queries when possible

### MQTT Bridge

- Adjust polling interval based on use case
- Consider using subscriptions for real-time updates
- Handle reconnection gracefully

### Memory Usage

- Use streaming for large payloads
- Clear old data when no longer needed
- Monitor memory in long-running bridges

## Security Notes

### Private Keys

- Never log or print private keys
- Use secure key storage (keyring, hardware wallet)
- Validate key formats before use

### Encryption

- Always verify recipient public key
- Use secure random number generation
- Validate nonce uniqueness

### Network

- Use TLS for production MQTT
- Validate WebSocket URLs
- Handle connection errors gracefully

## Future Improvements

Potential areas for enhancement:

1. **Metadata Caching**: Cache generated metadata to avoid regeneration
2. **Batch Operations**: Support creating multiple nodes at once
3. **Advanced Querying**: Add filtering and search capabilities
4. **Monitoring Dashboard**: Web UI for visualizing CPS trees
5. **Plugin System**: Allow custom command extensions
6. **Configuration File**: Support for `.cpsrc` config file
7. **Shell Completion**: Generate completions for bash/zsh/fish
8. **Docker Support**: Containerized deployment
9. **Metrics**: Export Prometheus metrics
10. **Webhooks**: HTTP callback support for events

## Resources

- [Robonomics Documentation](https://wiki.robonomics.network)
- [Subxt Documentation](https://docs.rs/subxt)
- [Substrate Documentation](https://docs.substrate.io)
- [MQTT Protocol](https://mqtt.org)
- [XChaCha20 Spec](https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-xchacha)

## License

Apache-2.0 - See LICENSE file for details
