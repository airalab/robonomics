#!/usr/bin/env bash

set -e

PROJECT_ROOT=`git rev-parse --show-toplevel`

export CARGO_INCREMENTAL=0

bold=$(tput bold)
normal=$(tput sgr0)

# Save current directory.
pushd . >/dev/null

cd $ROOT

for SRC in runtime/wasm
do
  echo "${bold}Building webassembly binary in $SRC...${normal}"
  cd "$PROJECT_ROOT/$SRC"

  ./build.sh

  cd - >> /dev/null
done

# Restore initial directory.
popd >/dev/null
