{ nixpkgs ? import ./nixpkgs.nix { }
, release ? import ./release.nix { }
}:

with nixpkgs;
with release;
with llvmPackages_latest;

stdenv.mkDerivation {
  name = "substrate-nix-shell";
  propagatedBuildInputs = [ msgs ];
  buildInputs = [ rustWasm wasm-gc ];
  LIBCLANG_PATH = "${libclang}/lib";
  PROTOC = "${protobuf}/bin/protoc";
}
