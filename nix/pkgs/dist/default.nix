{
  lib,
  stdenv,
  gnutar,
  libcps,
}:

stdenv.mkDerivation {
  name = "robonomics-distribution";
  dontUnpack = true;
  dontBuild = true;
  dontConfigure = true;
  installPhase = ''
    mkdir -p $out
    ${gnutar}/bin/tar -czvf $out/dist.tar.gz --transform='s|.*/||' ${libcps}/bin/cps
  '';
}
