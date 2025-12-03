#!/usr/bin/env bash

set -e

echo "*** Installing resolc stub (for CI testing)"

# Create a stub resolc binary that mimics the interface
# This is used for CI testing where actual resolc is not needed

cat > /tmp/resolc << 'EOF'
#!/usr/bin/env bash
# Stub resolc for CI testing
echo "resolc stub v1.0.0 (CI testing mode)"
exit 0
EOF

# Install the stub
chmod +x /tmp/resolc
sudo mv /tmp/resolc /usr/local/bin/resolc

# Verify installation
if command -v resolc &> /dev/null; then
    echo "resolc stub installed successfully"
    resolc --version || echo "resolc stub ready"
else
    echo "Failed to install resolc stub"
    exit 1
fi
