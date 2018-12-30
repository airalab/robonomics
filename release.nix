{ moz_overlay ? import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz)
, nixpkgs ? import <nixpkgs> { overlays = [ moz_overlay ]; }
}:

with nixpkgs.latest.rustChannels;
with nixpkgs;

let
  rust = nightly.rust.override {
    targets = [ "wasm32-unknown-unknown" ];
  };
  rustPlatform = nixpkgs.makeRustPlatform { callPackage = nixpkgs.callPackage; rustc = rust; cargo = nightly.cargo; };
in rec {
  package = nixpkgs.callPackage ./. { inherit rustPlatform; };
}
