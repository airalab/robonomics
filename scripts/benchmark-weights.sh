#!/bin/bash
# DEPRECATED: This script is deprecated in favor of the new benchmark workflow
# Please use the new benchmarking approach documented in the README:
#
# Quick start with Nix (recommended):
#   nix develop .#benchmarking -c ./scripts/benchmark-all.sh
#
# Or use the new script directly:
#   ./scripts/benchmark-all.sh
#
# See README.md for full documentation on runtime benchmarking.

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "⚠️  DEPRECATED: This script is deprecated"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Please use the new benchmarking workflow instead:"
echo ""
echo "  Quick start with Nix (recommended):"
echo "    nix develop .#benchmarking -c ./scripts/benchmark-all.sh"
echo ""
echo "  Or use the new script directly:"
echo "    ./scripts/benchmark-all.sh"
echo ""
echo "See README.md for full documentation on runtime benchmarking."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Ask if user wants to continue with the new script
read -p "Would you like to run the new benchmark script? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    exec "${SCRIPT_DIR}/benchmark-all.sh"
fi

exit 1

