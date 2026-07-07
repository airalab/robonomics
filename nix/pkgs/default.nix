{ pkgs }:

let
  musl = pkgs.pkgsCross.musl64;
  aarch64 = pkgs.pkgsCross.aarch64-multiplatform-musl;
  aarch64-musl = pkgs.pkgsCross.aarch64-multiplatform-musl;
in {
  robonomics = pkgs.callPackage ./robonomics {};
  robonomics-musl = musl.callPackage ./robonomics {};
  robonomics-aarch64 = aarch64.callPackage ./robonomics {};
  robonomics-aarch64-musl = aarch64-musl.callPackage ./robonomics {};

  libcps = pkgs.callPackage ./libcps {};
  libcps-musl = musl.callPackage ./libcps {};
  libcps-aarch64 = aarch64.callPackage ./libcps {};
  libcps-aarch64-musl = aarch64-musl.callPackage ./libcps {};

  robonet = pkgs.callPackage ./robonet {};
  robonet-musl = musl.callPackage ./robonet {};
  robonet-aarch64 = aarch64.callPackage ./robonet {};
  robonet-aarch64-musl = aarch64-musl.callPackage ./robonet {};
}
