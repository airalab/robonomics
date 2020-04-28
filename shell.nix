{ nixpkgs ? import ./nixpkgs.nix { }
, release ? import ./release.nix { }
}:

with nixpkgs;
with llvmPackages_latest;

stdenv.mkDerivation {
  name = "substrate-nix-shell";
  propagatedBuildInputs = [ release.substrate-ros-msgs ];
  buildInputs = [ release.rust wasm-bindgen-cli libudev pkgconfig ];
  LIBCLANG_PATH = "${libclang}/lib";
  PROTOC = "${protobuf}/bin/protoc";
}
