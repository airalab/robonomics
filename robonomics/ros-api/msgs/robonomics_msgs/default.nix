{ stdenv
, mkRosPackage
, substrate-ros-msgs
}:

mkRosPackage rec {
  name = "${pname}-${version}";
  pname = "robonomics_msgs";
  version = "master";

  src = ./.;

  propagatedBuildInputs = [ substrate-ros-msgs ];

  meta = with stdenv.lib; {
    description = "Robonomics module ROS messages";
    homepage = http://github.com/airalab/substrate-node-robonomics;
    license = licenses.asl20;
    maintainers = [ maintainers.akru ];
  };
}
