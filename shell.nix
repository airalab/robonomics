{ nixpkgs ? import ./nixpkgs.nix { } 
, release ? import ./release.nix { }
, ros ? true
}:

with nixpkgs;
with release;

stdenv.mkDerivation {
  name = "substrate-nix-shell";
  propagatedBuildInputs = if ros then [ msgs ] else [];
  buildInputs = [ rustWasm wasm-gc pkgconfig openssl clang ];
  LIBCLANG_PATH = "${llvmPackages.libclang}/lib";
  # FIXME: we can remove this once prost is updated.
  PROTOC = "${protobuf}/bin/protoc";
}
