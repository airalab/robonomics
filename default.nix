{ nixpkgs ? import ./nixpkgs.nix { }
, rustWasm
, msgs
}:

with nixpkgs;
with llvmPackages_latest;

rustPlatform.buildRustPackage rec {
  name = "robonomics-node";
  src = ./.;
  cargoSha256 = null; 
  propagatedBuildInputs = [ msgs ];
  buildInputs = [ rustWasm wasm-gc ];
  LIBCLANG_PATH = "${libclang}/lib";
}
