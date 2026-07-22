{
  revHash,
  lib,
  stdenv,
  gnutar,
  libcps,
  robonomics,
}:

stdenv.mkDerivation {
  name = "robonomics-package-${builtins.substring 0 7 revHash}";
  dontUnpack = true;
  dontBuild = true;
  dontConfigure = true;
  installPhase = ''
    mkdir -p $out
    ${gnutar}/bin/tar -czvf $out/package.tar.gz --transform='s|.*/||' ${libcps}/bin/cps ${robonomics}/bin/robonomics
    echo ${revHash} > $out/git-commit
  '';
}
