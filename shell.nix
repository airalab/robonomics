{ moz_overlay ? import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz)
, nixpkgs ? import <nixpkgs> { overlays = [ moz_overlay ]; }
}:

with nixpkgs.latest.rustChannels;
with nixpkgs;

let
  channel = rustChannelOf { date = "2019-01-08"; channel = "nightly"; };
  rust = channel.rust.override {
    targets = [ "wasm32-unknown-unknown" ];
  };
in
  stdenv.mkDerivation {
    name = "substrate-nix-shell";
    buildInputs = [ rust wasm-gc pkgconfig openssl clang ];
    LIBCLANG_PATH = "${llvmPackages.libclang}/lib";
  }
