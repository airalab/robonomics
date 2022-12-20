#!/usr/bin/env bash

set -e

head () {
    polkadot-js-api --ws ws://127.0.0.1:9944 query.system.number 2>/dev/null |\
        tail -3 |\
        jq -r .number
}

start=$(head)
sleep 60
end=$(head)

[ "$start" != "$end" ]
