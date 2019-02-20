Robonomics on Substrate
=======================

[![Build Status](https://travis-ci.org/airalab/substrate-node-robonomics.svg?branch=master)](https://travis-ci.org/airalab/substrate-node-robonomics)

> Substrate SRML-based Robonomics node Proof of Concept.

Quick start
-----------

Ensure you have Rust and the support software installed:

    curl https://sh.rustup.rs -sSf | sh
    # on Windows download and run rustup-init.exe
    # from https://rustup.rs instead

    rustup update nightly
    rustup target add wasm32-unknown-unknown --toolchain nightly
    rustup update stable
    cargo install --git https://github.com/alexcrichton/wasm-gc

You will also need to install the following packages:

* Linux: `sudo apt install cmake pkg-config libssl-dev git clang libclang-dev`
* Mac: `brew install cmake pkg-config openssl git llvm`
* Windows (PowerShell):

```
    # Install LLVM
    # Download and install the Pre Build Windows binaries
    # of LLVM  from http://releases.llvm.org/download.html
    
    # Install OpenSSL (through vcpkg)
    mkdir \Tools
    cd \Tools
    git clone https://github.com/Microsoft/vcpkg.git
    cd vcpkg
    .\bootstrap-vcpkg.bat
    .\vcpkg.exe install openssl:x64-windows-static
    
    $env:OPENSSL_DIR = 'C:\Tools\vcpkg\installed\x64-windows-static'
    $env:OPENSSL_STATIC = 'Yes'
    [System.Environment]::SetEnvironmentVariable('OPENSSL_DIR', $env:OPENSSL_DIR, [System.EnvironmentVariableTarget]::User)
    [System.Environment]::SetEnvironmentVariable('OPENSSL_STATIC', $env:OPENSSL_STATIC, [System.EnvironmentVariableTarget]::User)
```

Install robonomics node from git source:

    cargo install --force --git https://github.com/airalab/substrate-node-robonomics

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
