{ stdenv
, ros_comm
, mkRosPackage
, msgs
}:

mkRosPackage rec {
  name = "${pname}-${version}";
  pname = "turtlesim_liability";
  version = "master";

  src = ./.;

  propagatedBuildInputs = [ ros_comm msgs ];

  meta = with stdenv.lib; {
    description = "Robonomics Substrate Turtlesim example";
    homepage = http://github.com/airalab/substrate-node-robonomics;
    license = licenses.asl20;
    maintainers = [ maintainers.akru ];
  };
}
