{ nixpkgs ? import ./nixpkgs.nix { }
, ros ? true
, rustWasm
, msgs
}:

with nixpkgs;

rustPlatform.buildRustPackage rec {
  name = "robonomics-node";
  src = ./.;
  cargoSha256 = null; 
  propagatedBuildInputs = if ros then [ msgs ] else [];
  buildInputs = [ rustWasm wasm-gc pkgconfig openssl clang ];
  LIBCLANG_PATH = "${llvmPackages.libclang}/lib";
}
