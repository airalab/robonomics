#!/bin/sh

export RUSTC_VERSION=1.88.0
export PACKAGE=robonomics-runtime

docker run --rm -it -e PACKAGE=$PACKAGE -v $PWD:/build \
    -v $TMPDIR/cargo:/cargo-home paritytech/srtool:$RUSTC_VERSION
