# Robonomics Docker

This directory contains the Dockerfile and health check script for building lightweight Alpine-based Robonomics images.

## Using Pre-built Images

Pre-built multi-architecture images are available on Docker Hub:

```bash
docker pull robonomics/robonomics:latest
```

### Running a Node

```bash
docker run -d \
  --name robonomics \
  -p 30333:30333 \
  -p 9944:9944 \
  -p 9933:9933 \
  -v robonomics-data:/data \
  robonomics/robonomics:latest
```

### Available Ports

- `30333` - P2P port
- `30334` - P2P port (parachain)
- `9944` - WebSocket RPC
- `9945` - WebSocket RPC (parachain)
- `9933` - HTTP RPC
- `9934` - HTTP RPC (parachain)
- `9615` - Prometheus metrics
- `9616` - Prometheus metrics (parachain)

## Building Locally

To build the Docker image locally for x86_64:

```bash
# Build the Robonomics binary with musl target
cargo build --profile production --target x86_64-unknown-linux-musl

# Create architecture directory and copy binary
mkdir -p scripts/docker/amd64
cp target/x86_64-unknown-linux-musl/production/robonomics scripts/docker/amd64/

# Build Docker image
cd scripts/docker
docker build --build-arg TARGETARCH=amd64 -t robonomics/robonomics:local .
```

For aarch64 (ARM64) cross-compilation:

```bash
# Install cross-compilation tools
rustup target add aarch64-unknown-linux-musl

# Build the binary
cargo build --profile production --target aarch64-unknown-linux-musl

# Create architecture directory and copy binary
mkdir -p scripts/docker/arm64
cp target/aarch64-unknown-linux-musl/production/robonomics scripts/docker/arm64/

# Build Docker image
cd scripts/docker
docker build --build-arg TARGETARCH=arm64 -t robonomics/robonomics:local .
```

## Health Check

The container includes a health check that monitors chain progression by querying the RPC endpoint. The health check runs every 5 minutes and verifies that the block number is increasing.

