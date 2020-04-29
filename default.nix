{ nixpkgs ? import ./nixpkgs.nix { }
, rust
, substrate-ros-msgs
}:

with nixpkgs;
with llvmPackages_latest;

rustPlatform.buildRustPackage rec {
  name = "robonomics";
  src = ./.;
  cargoSha256 = null; 
  propagatedBuildInputs = [ substrate-msgs ];
  buildInputs = [ rust ];
  LIBCLANG_PATH = "${libclang}/lib";
}
