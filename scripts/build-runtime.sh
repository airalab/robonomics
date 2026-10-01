#!/bin/sh

if ! command -v docker >/dev/null 2>&1; then
    echo "Error: 'docker' not found in PATH" >&2
    echo "Install it from: https://docs.docker.com/get-docker/" >&2
    exit 1
fi

export RUSTC_VERSION=1.93.0
export PACKAGE=robonomics-runtime

docker run --rm -it -e PACKAGE=$PACKAGE -e BUILD_OPTS=$1 -v $PWD:/build \
    -v $TMPDIR/cargo:/cargo-home paritytech/srtool:$RUSTC_VERSION
