Robonomics on Substrate
=======================

[![Build Status](https://travis-ci.org/airalab/substrate-node-robonomics.svg?branch=master)](https://travis-ci.org/airalab/substrate-node-robonomics)

> Substrate SRML-based Robonomics node Proof of Concept.

Quick start
-----------

    cargo install --force --git https://github.com/airalab/substrate-node-robonomics robonomics

Run node in [Robonomics testnet](https://telemetry.polkadot.io/#/Robonomics)

    robonomics

Or run your local development network

    robonomics --dev

Nix way
-------

Install nix

    curl https://nixos.org/nix/install | sh

Run in Nix shell

    git clone https://github.com/airalab/substrate-node-robonomics && cd substrate-node-robonomics
    nix-shell --run "cargo run --release"

ROS integration
---------------

ROS integration module helps to use Robonomics Substrate module in ROS enabled cyber-physical systems.

### Building with ROS

Install ROS

    http://wiki.ros.org/melodic/Installation

Import ROS environment

    source /opt/ros/melodic/setup.bash

Build with `ros` feature

    cargo build --release --features ros

> Or just run in nix shell: `nix-shell --run "cargo run --release --features ros"`

### Launch

Start ROS core service

    roscore

Start node

    cargo run --release --features ros

Subscribe for best block number

    rostopic echo /blockchain/best_number
