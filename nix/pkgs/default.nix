{ pkgs, self }:

let
  # Cross-compilation static targets
  x86_64-llvm = pkgs.pkgsCross.musl64.pkgsLLVM;
  aarch64-llvm = pkgs.pkgsCross.aarch64-multiplatform-musl.pkgsLLVM;
  # Common parameters
  revHash = if (self ? rev) then self.rev else self.dirtyRev;
  protobuf-compiler = "${pkgs.protobuf}/bin/protoc";
in rec {
  default = robonomics;

  robonomics = pkgs.callPackage ./robonomics { inherit revHash protobuf-compiler; };
  robonomics-x86_64 = x86_64-llvm.callPackage ./robonomics { inherit revHash protobuf-compiler; };
  robonomics-aarch64 = aarch64-llvm.callPackage ./robonomics { inherit revHash protobuf-compiler; };

  libcps = pkgs.callPackage ./libcps {};
  libcps-x86_64 = x86_64-llvm.callPackage ./libcps {};
  libcps-aarch64 = aarch64-llvm.callPackage ./libcps {};

#  robonet = pkgs.callPackage ./robonet {};
#  robonet-static = x86_64-static.callPackage ./robonet {};
#  robonet-aarch64-static = aarch64-static.callPackage ./robonet {};

  runtime-metadata = pkgs.callPackage ./metadata {};

  package-x86_64 = pkgs.callPackage ./package {
    inherit revHash;
    libcps = libcps-x86_64;
    robonomics = robonomics-x86_64;
  };
  package-aarch64 = pkgs.callPackage ./package {
    inherit revHash;
    libcps = libcps-aarch64;
    robonomics = robonomics-aarch64;
  };
}
