#!/usr/bin/env bash

set -e

# Query the current block number via RPC
get_block_number() {
    curl -sS -H "Content-Type: application/json" \
         -d '{"id":1, "jsonrpc":"2.0", "method":"chain_getHeader"}' \
         http://127.0.0.1:9944 | jq -r '.result.number' | sed 's/^0x//' | xargs printf "%d"
}

start=$(get_block_number)
sleep 60
end=$(get_block_number)

# Check if block number has increased (chain is progressing)
[ "$start" != "$end" ]
