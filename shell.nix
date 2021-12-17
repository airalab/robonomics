{ release ? import ./release.nix { }
}:

with release.pkgs;
with llvmPackages;

stdenv.mkDerivation {
  name = "robonomics-nix-shell";
  nativeBuildInputs = [ clang zlib ];
  buildInputs = [ release.rust-nightly ];
  LIBCLANG_PATH = "${clang-unwrapped.lib}/lib";
  PROTOC = "${protobuf}/bin/protoc";
}
