Robonomics on Substrate
=======================

> Substrate SRML-based Robonomics node Proof of Concept.

Quick start
-----------

## Classic install

### Install Rust

    curl https://sh.rustup.rs -sSf | sh -s -- -y
    source ~/.cargo/env

### Install Robonomics node

    cargo install --force --git https://github.com/airalab/substrate-node-robonomics robonomics
    robonomics

## Nix way install

### Install nix

    curl https://nixos.org/nix/install | sh`

### Run in Nix shell

    git clone https://github.com/airalab/substrate-node-robonomics && cd substrate-node-robonomics
    nix-shell --run "cargo run --release"`

