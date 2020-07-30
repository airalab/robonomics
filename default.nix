{ nixpkgs ? import ./nixpkgs.nix { }
, rust
, substrate-ros-msgs
}:

with nixpkgs;
with llvmPackages_latest;

let
  pname = "robonomics";
in rustPlatform.buildRustPackage {
  name = pname;
  src = builtins.path { path = ./.; name = pname; };
  cargoSha256 = null; 
  buildInputs = [ substrate-ros-msgs rust libudev pkgconfig ];
  LIBCLANG_PATH = "${libclang}/lib";
  PROTOC = "${protobuf}/bin/protoc";
}
