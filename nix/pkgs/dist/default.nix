{
  lib,
  stdenv,
  gnutar,
  libcps,
  robonomics,
}:

stdenv.mkDerivation {
  name = "robonomics-distribution";
  dontUnpack = true;
  dontBuild = true;
  dontConfigure = true;
  installPhase = ''
    mkdir -p $out
    ${gnutar}/bin/tar -czvf $out/dist.tar.gz --transform='s|.*/||' ${libcps}/bin/cps ${robonomics}/bin/robonomics
  '';
}
