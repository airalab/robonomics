{ pkgs, self }:

let
  # Cross-compilation static targets
  x86_64-llvm = pkgs.pkgsCross.musl64.pkgsLLVM;
  aarch64-llvm = pkgs.pkgsCross.aarch64-multiplatform-musl.pkgsLLVM;
  # Common parameters
  revHash = if (self ? rev) then self.rev else self.dirtyRev;
in rec {
  default = robonomics;

  robonomics = pkgs.callPackage ./robonomics { inherit revHash; };
  libcps = pkgs.callPackage ./libcps {};

  runtime-metadata = pkgs.callPackage ./metadata {};
}
