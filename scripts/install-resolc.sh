#!/usr/bin/env bash

set -e

echo "*** Installing resolc stub (for CI testing)"

# Create a stub resolc binary that mimics the interface
# This is used for CI testing where actual resolc is not needed

cat > /tmp/resolc << 'EOF'
#!/usr/bin/env bash
# Stub resolc for CI testing

# Handle common flags
case "${1:-}" in
  --version|-V)
    echo "resolc stub v1.0.0 (CI testing mode)"
    exit 0
    ;;
  --help|-h)
    echo "resolc stub - CI testing mode"
    echo "This is a stub implementation for CI testing purposes."
    exit 0
    ;;
  *)
    # Default: accept any arguments and exit successfully
    exit 0
    ;;
esac
EOF

# Install the stub
chmod +x /tmp/resolc
sudo mv /tmp/resolc /usr/local/bin/resolc

# Verify installation
if command -v resolc &> /dev/null; then
    echo "resolc stub installed successfully"
    resolc --version
else
    echo "Failed to install resolc stub"
    exit 1
fi
