{ nixpkgs ? import ./nixpkgs.nix { }
}:

with nixpkgs;

let
  channel = rustChannelOf { date = "2019-09-03"; channel = "nightly"; };
  targets = [ "wasm32-unknown-unknown" ];

in rec {
  msgs = callPackage ./substrate-ros/msgs/substrate_ros_msgs { };
  turtlesim = callPackage ./examples/turtlesim_liability { inherit msgs; };
  ros_tutorials = callPackage ./examples/ros_tutorials { };
  rustWasm = channel.rust.override { inherit targets; };
  node = callPackage ./. { inherit rustWasm msgs; };
}
