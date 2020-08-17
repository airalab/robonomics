{ stdenv
, catkin 
, buildRosPackage 
, message-generation
, rospy 
, std-msgs
}:

buildRosPackage rec {
  name = "${pname}-${version}";
  pname = "turtlesim_liability";
  version = "master";

  src = ./.;

  buildType = "catkin";
  buildInputs = [ message-generation ];
  propagatedBuildInputs = [ rospy std-msgs ];
  nativeBuildInputs = [ catkin ];

  meta = with stdenv.lib; {
    description = "Robonomics Substrate Turtlesim example";
    homepage = http://github.com/airalab/substrate-node-robonomics;
    license = licenses.asl20;
    maintainers = [ maintainers.akru ];
  };
}
