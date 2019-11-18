{ nixpkgs ? import ./nixpkgs.nix { }
, release ? import ./release.nix { }
}:

with nixpkgs;
with llvmPackages_latest;

stdenv.mkDerivation {
  name = "substrate-nix-shell";
  propagatedBuildInputs = [ release.msgs ];
  buildInputs = [ release.rust wasm-gc ];
  LIBCLANG_PATH = "${libclang}/lib";
  PROTOC = "${protobuf}/bin/protoc";
}
