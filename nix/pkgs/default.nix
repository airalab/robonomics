{ pkgs, self }:

let
  # Cross-compilation static targets
  musl = pkgs.pkgsCross.musl64.pkgsStatic.pkgsLLVM;
  aarch64-musl = pkgs.pkgsCross.aarch64-multiplatform-musl.pkgsStatic.pkgsLLVM;
  # Common parameters
  revHash = if (self ? rev) then self.rev else self.dirtyRev;
  protobuf-compiler = "${pkgs.protobuf}/bin/protoc";
in rec {
  default = robonomics;

  robonomics = pkgs.callPackage ./robonomics { inherit revHash protobuf-compiler; };
  robonomics-musl = musl.callPackage ./robonomics { inherit revHash protobuf-compiler; };
  robonomics-aarch64-musl = aarch64-musl.callPackage ./robonomics { inherit revHash protobuf-compiler; };

  libcps = pkgs.callPackage ./libcps {};
  libcps-musl = musl.callPackage ./libcps {};
  libcps-aarch64-musl = aarch64-musl.callPackage ./libcps {};

  robonet = pkgs.callPackage ./robonet {};
  robonet-musl = musl.callPackage ./robonet {};
  robonet-aarch64-musl = aarch64-musl.callPackage ./robonet {};

  package-x86_64 = pkgs.callPackage ./package {
    inherit revHash;
    libcps = libcps-musl;
    robonomics = robonomics-musl;
  };
  package-aarch64 = pkgs.callPackage ./package {
    inherit revHash;
    libcps = libcps-aarch64-musl;
    robonomics = robonomics-aarch64-musl;
  };

  runtime-metadata = pkgs.callPackage ./metadata {};
}
