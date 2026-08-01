{ pkgs, self }:

let
  # Cross-compilation static targets
  x86_64-static = pkgs.pkgsCross.musl64.pkgsLLVM;
  aarch64-static = pkgs.pkgsCross.aarch64-multiplatform-musl.pkgsLLVM;
  riscv64-static = pkgs.pkgsCross.riscv64.pkgsStatic.pkgsLLVM;
  # Common parameters
  revHash = if (self ? rev) then self.rev else self.dirtyRev;
  protobuf-compiler = "${pkgs.protobuf}/bin/protoc";
in rec {
  default = robonomics;

  robonomics = pkgs.callPackage ./robonomics { inherit revHash protobuf-compiler; };
  robonomics-x86_64-static = x86_64-static.callPackage ./robonomics { inherit revHash protobuf-compiler; };
  robonomics-aarch64-static = aarch64-static.callPackage ./robonomics { inherit revHash protobuf-compiler; };
  robonomics-riscv64-static = riscv64-static.callPackage ./robonomics { inherit revHash protobuf-compiler; };

  libcps = pkgs.callPackage ./libcps {};
  libcps-x86_64-static = x86_64-static.callPackage ./libcps {};
  libcps-aarch64-static = aarch64-static.callPackage ./libcps {};
  libcps-riscv64-static = riscv64-static.callPackage ./libcps {};

#  robonet = pkgs.callPackage ./robonet {};
#  robonet-static = x86_64-static.callPackage ./robonet {};
#  robonet-aarch64-static = aarch64-static.callPackage ./robonet {};

  runtime-metadata = pkgs.callPackage ./metadata {};

  package-x86_64 = pkgs.callPackage ./package {
    inherit revHash;
    libcps = libcps-x86_64-static;
    robonomics = robonomics-x86_64-static;
  };
  package-aarch64 = pkgs.callPackage ./package {
    inherit revHash;
    libcps = libcps-aarch64-static;
    robonomics = robonomics-aarch64-static;
  };
  package-riscv64 = pkgs.callPackage ./package {
    inherit revHash;
    libcps = libcps-riscv64-static;
    robonomics = robonomics-riscv64-static;
  };
}
