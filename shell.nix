{ moz_overlay ? import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz)
, nixpkgs ? import (builtins.fetchTarball https://github.com/airalab/airapkgs/archive/nixos-unstable.tar.gz) { overlays = [ moz_overlay ]; }
}:

with nixpkgs.latest.rustChannels;
with nixpkgs;

let
  channel = rustChannelOf { date = "2019-07-01"; channel = "nightly"; };
  rust = channel.rust.override { targets = [ "wasm32-unknown-unknown" ]; };
  msgs = callPackage ./substrate-ros/msgs/substrate_ros_msgs { };
in
  stdenv.mkDerivation {
    name = "substrate-nix-shell";
    buildInputs = [
      rust pkgconfig openssl clang msgs
    ];
    LIBCLANG_PATH = "${llvmPackages.libclang}/lib";
  }
