#!/usr/bin/env bash

set -e

head () {
    polkadot-js-api --ws ws://127.0.0.1:9944 query.system.number |\
        jq -r .number
}

start=$(head)
sleep 60
end=$(head)

[ "$start" != "$end" ]
