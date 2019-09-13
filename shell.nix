{ nixpkgs ? import ./nixpkgs.nix { }
, release ? import ./release.nix { }
}:

with nixpkgs;
with release;
with llvmPackages_latest;

stdenv.mkDerivation {
  name = "substrate-nix-shell";
  propagatedBuildInputs = [ msgs ];
  buildInputs = [ rustWasm wasm-gc pkgconfig openssl clang ];
  LIBCLANG_PATH = "${libclang}/lib";
  # FIXME: we can remove this once prost is updated.
  PROTOC = "${protobuf}/bin/protoc";
}
