# Development Guide

This document provides information for developers working on the libcps library and CPS CLI tool.

## Prerequisites

- Rust 1.88.0 or later
- A running Robonomics node with CPS pallet for testing
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
- `mqtt = ["dep:rumqttc", "dep:toml"]`
- `cli = ["mqtt", "clap", "colored", "chrono", "indicatif", "env_logger", "easy-hex"]`

## Code Structure

### Main Components

1. **`src/lib.rs`**: Library entry point with module exports
2. **`src/main.rs`**: CLI entry point using `clap` for argument parsing
3. **`src/blockchain/`**: Blockchain client and connection management
4. **`src/commands/`**: Individual CLI command implementations (thin wrappers)
5. **`src/crypto/`**: Encryption/decryption utilities (library)
6. **`src/display/`**: Beautiful colored output formatting (CLI-only)
7. **`src/mqtt/`**: MQTT bridge configuration and implementation (library)
8. **`src/node.rs`**: Node-oriented API with type definitions for CPS operations (library)

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
    display::progress("Executing my command...");
    let client = Client::new(config).await?;
    // Your implementation here
    display::success("Command completed!");
    Ok(())
}
```

## Blockchain Metadata

libcps extracts metadata directly from the robonomics runtime at build time. This approach brings **much less dependencies** than embedding the runtime WASM in the subxt macro.

### How It Works

The `build.rs` script extracts metadata from the runtime and saves it to the build directory:

1. **Load runtime WASM**: Gets `WASM_BINARY` from robonomics-runtime build dependency
2. **Create RuntimeBlob**: Prepares the WASM for execution
3. **Execute metadata call**: Uses `WasmExecutor` to call the `Metadata_metadata` host function
4. **Decode and validate**: Decodes SCALE-encoded metadata and validates magic bytes
5. **Save to file**: Writes metadata to `$OUT_DIR/metadata.scale`
6. **Subxt macro**: Reads the metadata file at compile time to generate type-safe APIs

### Benefits

- **Fewer dependencies**: No need to embed runtime WASM or pull in heavy runtime dependencies
- **Faster builds**: Metadata extraction happens once during build
- **Always in sync**: Metadata comes directly from runtime dependency version
- **Type safe**: Compile-time verification of all runtime calls
- **Self-contained**: Everything happens in the build process

### Build Dependencies

The metadata extraction requires these dependencies (build-time only):

```toml
[build-dependencies]
robonomics-runtime = { workspace = true }
sp-io = { workspace = true }
sp-state-machine = { workspace = true }
sc-executor = { workspace = true }
sc-executor-common = { workspace = true }
parity-scale-codec = { workspace = true }
```

These are only needed during compilation and don't bloat the final binary.

## Testing

### Unit Tests

```bash
cargo test --package libcps
```

### Integration Testing

For integration testing with a live node, see the README for setup instructions.

## Code Quality

### Linting

```bash
# Run clippy
cargo clippy --package libcps

# Apply automatic fixes
cargo clippy --fix --package libcps --allow-dirty
```

### Formatting

```bash
# Check formatting
cargo fmt --package libcps -- --check

# Apply formatting
cargo fmt --package libcps
```

## Debugging

### Enable Rust logging

```bash
export RUST_LOG=debug
cargo run --package libcps -- show 0
```

### Using LLDB/GDB

```bash
# Build with debug symbols
cargo build --package libcps

# Run with debugger
rust-lldb target/debug/cps
# or
rust-gdb target/debug/cps
```

## Dependencies

### Core Dependencies

- `subxt`: Substrate RPC client
- `subxt-signer`: Account signing utilities
- `clap`: Command-line argument parsing (CLI only)
- `tokio`: Async runtime
- `anyhow`: Error handling

### Crypto Dependencies

- `curve25519-dalek`: Elliptic curve operations
- `x25519-dalek`: Key exchange
- `chacha20poly1305`: XChaCha20-Poly1305 encryption
- `aes-gcm`: AES-GCM encryption
- `hkdf`: HMAC-based key derivation
- `sha2`: SHA-256 hashing

### UI Dependencies (CLI only)

- `colored`: Terminal colors and formatting
- `indicatif`: Progress bars
- `serde`/`serde_json`: Serialization

### Optional Dependencies

- `rumqttc`: MQTT client (feature: `mqtt`)

## Architecture Decisions

### Why Extract Metadata at Build Time?

**Problem**: Embedding runtime WASM directly in the code brings many heavy dependencies.

**Solution**: Extract metadata once during build and save it to a file. This:
- Reduces compile-time dependencies significantly
- Makes builds faster after initial metadata extraction
- Keeps the final binary smaller
- Still ensures metadata is always in sync with runtime version

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

- **Modularity**: Each command is self-contained
- **Testability**: Easy to test individual commands
- **Maintainability**: Clear code organization
- **Extensibility**: Easy to add new commands

### Library vs CLI Split

The codebase is organized to separate library functionality from CLI:

- **Library code** (`lib.rs`, `blockchain/`, `crypto/`, `mqtt/`, `node.rs`): Pure functionality, no colored output
- **CLI code** (`main.rs`, `commands/`, `display/`): User interface, pretty printing, argument parsing

This allows:
- Using libcps as a library without CLI overhead
- Building custom tools on top of libcps
- Optional features for different use cases

## Common Issues

### Connection Refused

**Problem**: `Error when opening the TCP socket: Connection refused`

**Solution**: Make sure your Robonomics node is running and the WebSocket endpoint is correct.

### Metadata Build Errors

**Problem**: Build fails during metadata extraction

**Solution**: 
1. Clean the build: `cargo clean -p libcps`
2. Ensure robonomics-runtime dependency is up to date
3. Check that all build-dependencies are available
4. Rebuild: `cargo build -p libcps`

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

## Resources

- [Robonomics Documentation](https://wiki.robonomics.network)
- [Subxt Documentation](https://docs.rs/subxt)
- [Substrate Documentation](https://docs.substrate.io)
- [MQTT Protocol](https://mqtt.org)
- [XChaCha20 Spec](https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-xchacha)

## License

Apache-2.0 - See LICENSE file for details