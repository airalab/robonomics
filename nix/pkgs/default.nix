{ pkgs, revHash }:

let
  musl = pkgs.pkgsCross.musl64;
  aarch64 = pkgs.pkgsCross.aarch64-multiplatform-musl;
  aarch64-musl = pkgs.pkgsCross.aarch64-multiplatform-musl;
in rec {
  default = robonomics;

  robonomics = pkgs.callPackage ./robonomics { inherit revHash; };
  robonomics-musl = musl.callPackage ./robonomics { inherit revHash; };
  robonomics-aarch64 = aarch64.callPackage ./robonomics {inherit revHash; };
  robonomics-aarch64-musl = aarch64-musl.callPackage ./robonomics { inherit revHash; };

  libcps = pkgs.callPackage ./libcps {};
  libcps-musl = musl.callPackage ./libcps {};
  libcps-aarch64 = aarch64.callPackage ./libcps {};
  libcps-aarch64-musl = aarch64-musl.callPackage ./libcps {};

  robonet = pkgs.callPackage ./robonet {};
  robonet-musl = musl.callPackage ./robonet {};
  robonet-aarch64 = aarch64.callPackage ./robonet {};
  robonet-aarch64-musl = aarch64-musl.callPackage ./robonet {};

  dist-x86_64 = pkgs.callPackage ./dist {
    libcps = libcps-musl;
    robonomics = robonomics-musl;
  };
  dist-aarch64 = pkgs.callPackage ./dist {
    libcps = libcps-aarch64-musl;
    robonomics = robonomics-aarch64-musl;
  };

  runtime-metadata = pkgs.callPackage ./metadata {};
}
