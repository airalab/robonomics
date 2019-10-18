{ nixpkgs ? import ./nixpkgs.nix { }
}:

with nixpkgs;

let
  channel = rustChannelOf { date = "2019-09-03"; channel = "nightly"; };

in rec {
  rustWasm = channel.rust.override {
    targets = [ "wasm32-unknown-unknown" ];
  };
  msgs = callPackage ./substrate-ros/msgs/substrate_ros_msgs { };
  turtlesim = callPackage ./examples/turtlesim_liability { inherit msgs; };
  node = callPackage ./. { inherit rustWasm msgs; };
}
