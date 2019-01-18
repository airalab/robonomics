Robonomics on Substrate
=======================

[![Build Status](https://travis-ci.org/airalab/substrate-node-robonomics.svg?branch=master)](https://travis-ci.org/airalab/substrate-node-robonomics)

> Substrate SRML-based Robonomics node Proof of Concept.

Quick start
-----------

    cargo install --force --git https://github.com/airalab/substrate-node-robonomics robonomics

Run node in [Robonomics testnet](https://telemetry.polkadot.io/#/Robonomics):

    robonomics

Or run your local development network:

    robonomics --dev

Nix way
-------

### Install nix

    curl https://nixos.org/nix/install | sh

### Run in Nix shell

    git clone https://github.com/airalab/substrate-node-robonomics && cd substrate-node-robonomics
    nix-shell --run "cargo run --release"

