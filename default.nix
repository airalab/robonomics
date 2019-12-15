{ nixpkgs ? import ./nixpkgs.nix { }
, rust
, substrate-ros-msgs
, robonomics-msgs
}:

with nixpkgs;
with llvmPackages_latest;

rustPlatform.buildRustPackage rec {
  name = "robonomics";
  src = ./.;
  cargoSha256 = null; 
  propagatedBuildInputs = [ substrate-msgs robonomics-msgs ];
  buildInputs = [ rust wasm-gc ];
  LIBCLANG_PATH = "${libclang}/lib";
}
