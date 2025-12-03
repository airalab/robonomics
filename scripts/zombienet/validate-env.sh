#!/usr/bin/env bash
#
# Environment validation script for zombienet tests
# Checks if all prerequisites are met before running tests
#

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

CHECKS_PASSED=0
CHECKS_FAILED=0
WARNINGS=0

# Check functions
check_pass() {
    echo -e "${GREEN}✓${NC} $1"
    ((CHECKS_PASSED++))
}

check_fail() {
    echo -e "${RED}✗${NC} $1"
    ((CHECKS_FAILED++))
}

check_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
    ((WARNINGS++))
}

echo "======================================"
echo "Zombienet Environment Validation"
echo "======================================"
echo ""

# Check Node.js
echo "[1/10] Checking Node.js..."
if command -v node >/dev/null 2>&1; then
    NODE_VERSION=$(node --version)
    NODE_MAJOR=$(echo "$NODE_VERSION" | cut -d'.' -f1 | tr -d 'v')
    if [ "$NODE_MAJOR" -ge 16 ]; then
        check_pass "Node.js $NODE_VERSION (>= 16.x required)"
    else
        check_fail "Node.js $NODE_VERSION (>= 16.x required)"
    fi
else
    check_fail "Node.js not found"
fi

# Check npm or yarn
echo "[2/10] Checking package manager..."
if command -v yarn >/dev/null 2>&1; then
    check_pass "yarn $(yarn --version)"
elif command -v npm >/dev/null 2>&1; then
    check_pass "npm $(npm --version)"
else
    check_fail "npm or yarn not found"
fi

# Check Rust
echo "[3/10] Checking Rust..."
if command -v rustc >/dev/null 2>&1; then
    check_pass "rustc $(rustc --version | awk '{print $2}')"
else
    check_fail "Rust not found"
fi

# Check cargo
echo "[4/10] Checking Cargo..."
if command -v cargo >/dev/null 2>&1; then
    check_pass "cargo $(cargo --version | awk '{print $2}')"
else
    check_fail "Cargo not found"
fi

# Check protobuf compiler
echo "[5/10] Checking protobuf compiler..."
if command -v protoc >/dev/null 2>&1; then
    check_pass "protoc $(protoc --version | awk '{print $2}')"
else
    check_warn "protoc not found (required for building Robonomics)"
fi

# Check Robonomics binary
echo "[6/10] Checking Robonomics binary..."
# Check for production profile binary first, fall back to release
if [ -f "${PROJECT_ROOT}/target/production/robonomics" ]; then
    ROBONOMICS_BIN="${PROJECT_ROOT}/target/production/robonomics"
    check_pass "Robonomics binary exists at $ROBONOMICS_BIN (production profile)"
elif [ -f "${PROJECT_ROOT}/target/release/robonomics" ]; then
    ROBONOMICS_BIN="${PROJECT_ROOT}/target/release/robonomics"
    check_pass "Robonomics binary exists at $ROBONOMICS_BIN (release profile)"
else
    check_warn "Robonomics binary not found (will be built if needed)"
fi

# Check available disk space
echo "[7/10] Checking disk space..."
# Using df -Pk and robust parsing for portability across Linux and macOS
AVAILABLE_SPACE=$(df -Pk "$PROJECT_ROOT" | awk '
    NR==1 {
        for (i=1; i<=NF; i++) {
            if ($i == "Available") avail_col=i
        }
    }
    NR==2 && avail_col {
        print int($avail_col/1024/1024)
    }
')
if [ "$AVAILABLE_SPACE" -ge 10 ]; then
    check_pass "Sufficient disk space: ${AVAILABLE_SPACE}GB available"
else
    check_warn "Low disk space: ${AVAILABLE_SPACE}GB available (10GB+ recommended)"
fi

# Check available memory
echo "[8/10] Checking memory..."
if command -v free >/dev/null 2>&1; then
    AVAILABLE_MEM=$(free -g | awk '
        NR==1 {
            for (i=1; i<=NF; i++) {
                if ($i == "available") {
                    avail_col = i
                    break
                }
            }
        }
        NR==2 && avail_col {
            print $avail_col
        }
    ')
    if [ -n "$AVAILABLE_MEM" ] && [ "$AVAILABLE_MEM" -ge 4 ]; then
        check_pass "Sufficient memory: ${AVAILABLE_MEM}GB available"
    else
        check_warn "Low memory: ${AVAILABLE_MEM}GB available (4GB+ recommended)"
    fi
else
    check_warn "Cannot check memory (free command not available)"
fi

# Check port availability
echo "[9/10] Checking port availability..."
PORTS_IN_USE=0
# Check ports using available tools (lsof, netstat, or ss)
for PORT in 9944 9945 9988 9989 30333 30334 31200 31201; do
    port_in_use=false
    
    # Try lsof if available
    if command -v lsof >/dev/null 2>&1; then
        if lsof -Pi :$PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
            port_in_use=true
        fi
    fi
    
    # Try netstat as fallback
    if [ "$port_in_use" = false ] && command -v netstat >/dev/null 2>&1; then
        if netstat -tuln 2>/dev/null | grep -q ":$PORT "; then
            port_in_use=true
        fi
    fi
    
    if [ "$port_in_use" = true ]; then
        check_warn "Port $PORT is already in use"
        ((PORTS_IN_USE++))
    fi
done

if [ $PORTS_IN_USE -eq 0 ]; then
    check_pass "All required ports are available"
fi

# Check curl availability
echo "[10/10] Checking curl..."
if command -v curl >/dev/null 2>&1; then
    check_pass "curl $(curl --version | head -n1 | awk '{print $2}')"
else
    check_fail "curl not found (required for downloading binaries)"
fi

# Summary
echo ""
echo "======================================"
echo "Validation Summary"
echo "======================================"
echo -e "${GREEN}Passed:${NC}   $CHECKS_PASSED"
echo -e "${YELLOW}Warnings:${NC} $WARNINGS"
echo -e "${RED}Failed:${NC}   $CHECKS_FAILED"
echo ""

if [ $CHECKS_FAILED -gt 0 ]; then
    echo -e "${RED}Environment validation failed!${NC}"
    echo "Please install missing dependencies before running tests."
    exit 1
elif [ $WARNINGS -gt 0 ]; then
    echo -e "${YELLOW}Environment validation passed with warnings.${NC}"
    echo "Tests may run, but some issues might occur."
    exit 0
else
    echo -e "${GREEN}Environment validation passed!${NC}"
    echo "You can now run: ./run-tests.sh"
    exit 0
fi
