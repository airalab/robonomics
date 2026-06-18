{ pkgs }:

{
  robonomics = pkgs.callPackage ./robonomics { };
  #robonet = pkgs.callPackage ./robonet { };
  #libcps = pkgs.callPackage ./libcps { };
}
