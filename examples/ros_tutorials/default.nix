{ stdenv
, fetchFromGitHub
, mkRosPackage
, python3Packages
, geometry_msgs
, ros_comm
, qt5
}:

mkRosPackage rec {
  name = "${pname}-${version}";
  pname = "ros_tutorials";
  version = "0.9.1";

  src = fetchFromGitHub {
    owner = "ros";
    repo = pname;
    rev = version;
    sha256 = "0m1niax6566jgz6iz9ypjz90kndprv1iz159lkcxbgs4jxi3zxi7";
  };

  propagatedBuildInputs = with python3Packages;
  [ ros_comm geometry_msgs qt5.full ];

  meta = with stdenv.lib; {
    description = "Packages that demonstrate various features of ROS.";
    homepage = http://wiki.ros.org/ros_tutorials;
    license = licenses.bsd3;
    maintainers = [ maintainers.akru ];
  };
}
