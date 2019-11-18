{ nixpkgs ? import ./nixpkgs.nix { }
, rust
, msgs
}:

with nixpkgs;
with llvmPackages_latest;

rustPlatform.buildRustPackage rec {
  name = "robonomics";
  src = ./.;
  cargoSha256 = null; 
  propagatedBuildInputs = [ msgs ];
  buildInputs = [ rust wasm-gc ];
  LIBCLANG_PATH = "${libclang}/lib";
}
