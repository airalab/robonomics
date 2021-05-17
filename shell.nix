{ release ? import ./release.nix { }
}:

with release.pkgs;
with llvmPackages;

stdenv.mkDerivation {
  name = "robonomics-nix-shell";
  propagatedBuildInputs = [ release.substrate-ros-msgs ];
  nativeBuildInputs = [ clang ];
  buildInputs = [ release.rust-nightly ];
  LIBCLANG_PATH = "${libclang}/lib";
  PROTOC = "${protobuf}/bin/protoc";
}
