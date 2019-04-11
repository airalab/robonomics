{ moz_overlay ? import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz)
, nixpkgs ? import (builtins.fetchTarball https://github.com/airalab/airapkgs/archive/nixos-unstable.tar.gz) { overlays = [ moz_overlay ]; }
}:

with nixpkgs.latest.rustChannels;
with nixpkgs;

let
  channel = rustChannelOf { date = "2019-04-11"; channel = "nightly"; };
  rust = channel.rust.override { targets = [ "wasm32-unknown-unknown" ]; };
  msgs = callPackage ./robonomics_msgs { };
in
  stdenv.mkDerivation {
    name = "substrate-nix-shell";
    buildInputs = [
      rust msgs wasm-gc pkgconfig openssl clang
    ];
    LIBCLANG_PATH = "${llvmPackages.libclang}/lib";
  }
