{ pkgs, self }:

let
  # Cross-compilation static targets
  x86_64-llvm = pkgs.pkgsCross.musl64.pkgsLLVM;
  aarch64-llvm = pkgs.pkgsCross.aarch64-multiplatform-musl.pkgsLLVM;
  riscv64-llvm = pkgs.pkgsCross.riscv64.pkgsLLVM;
  # Common parameters
  revHash = if (self ? rev) then self.rev else self.dirtyRev;
  protobuf-compiler = "${pkgs.protobuf}/bin/protoc";
in rec {
  default = robonomics;

  robonomics = pkgs.callPackage ./robonomics { inherit revHash protobuf-compiler; };
  robonomics-x86_64-static = x86_64-llvm.callPackage ./robonomics { inherit revHash protobuf-compiler; isStatic = true; };
  robonomics-aarch64-static = aarch64-llvm.callPackage ./robonomics { inherit revHash protobuf-compiler; isStatic = true; };
  robonomics-riscv64-static = riscv64-llvm.callPackage ./robonomics { inherit revHash protobuf-compiler; isStatic = true; };

  libcps = pkgs.callPackage ./libcps {};
  libcps-x86_64-static = x86_64-llvm.callPackage ./libcps { isStatic = true; };
  libcps-aarch64-static = aarch64-llvm.callPackage ./libcps { isStatic = true; };
  libcps-riscv64-static = riscv64-llvm.callPackage ./libcps { isStatic = true; };

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
