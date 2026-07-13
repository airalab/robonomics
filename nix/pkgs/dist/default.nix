{
  lib,
  stdenv,
  pkgs,
  target ? "x86_64",
}:

stdenv.mkDerivation {
  name = "robonomics-dist-${target}";
  builder = "${pkgs.bash}/bin/bash";
  args = with pkgs; ["-c" ''
    mkdir -p $out
    ${pkgs.gnutar}/bin/tar -czvf $out/dist.tar.gz ${libcps-musl}/bin/cps
  ''];
}
