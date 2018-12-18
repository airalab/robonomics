{ moz_overlay ? import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz)
, nixpkgs ? import <nixpkgs> { overlays = [ moz_overlay ]; }
}:

with nixpkgs.latest.rustChannels;
with nixpkgs;

let
  rust = nightly.rust.override {
    targets = [ "wasm32-unknown-unknown" ];
  };
  cargo = nightly.cargo;
in
  stdenv.mkDerivation {
    name = "substrate-nix-shell";
    buildInputs = [ rust cargo wasm-gc pkgconfig openssl ];
  }
