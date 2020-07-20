{ nixpkgs ? import (builtins.fetchTarball https://github.com/nixos/nixpkgs-channels/archive/nixos-unstable.tar.gz)
, mozilla_rust ? builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz
, nix_ros_overlay ? builtins.fetchTarball https://github.com/lopsided98/nix-ros-overlay/archive/master.tar.gz
}:

let
  moz_overlay = import mozilla_rust;
  ros_overlay = import "${nix_ros_overlay}/overlay.nix";
in nixpkgs {
  overlays = [ moz_overlay ros_overlay ];
}
