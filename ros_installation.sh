#!/bin/bash

function usage {
    # Print out usage of this script.
    echo >&2 "usage: $0 [catkin workspace name (default:catkin_ws)] [ROS distro (default: kinetic)"
    exit 0
}

function get-shell {
    current_user=`id -u -n`
    shell=`cat /etc/passwd | grep $current_user | cut -d ':' -f 7`
    echo ${shell##*/}
}

name_catkinws=$1
name_catkinws=${name_catkinws:="ros_ws"}
name_ros_distro=$2
name_ros_distro=${name_ros_distro:="melodic"}

echo $name_catkinws
echo $name_ros_distro

username=`id -u -n`

user_shell=`get-shell`

if [ -d /etc/upstream-release ] ; then
    relesenum=`cat /etc/upstream-release/lsb-release | grep DESCRIPTION | awk -F 'Ubuntu ' '{print $2}' | awk -F ' LTS' '{print $1}'`
    ubuntu_version=`cat /etc/upstream-release/lsb-release | grep CODENAME | awk -F '=' '{print $2}'`
else
    relesenum=`cat /etc/lsb-release | grep DESCRIPTION | awk -F 'Ubuntu ' '{print $2}' | awk -F ' LTS' '{print $1}'`
    ubuntu_version=`cat /etc/lsb-release | grep CODENAME | awk -F '=' '{print $2}'`
fi

its_okay_to_install=true
package_type="ros-base"

echo "[Update & upgrade the package]"
sudo apt-get update
sudo apt-get upgrade -y

echo "[Installing chrony and setting the ntpdate]"
sudo apt-get install -y chrony ntpdate
sudo ntpdate ntp.ubuntu.com

echo "[Add the ROS repository]"
sudo sh -c 'echo "deb http://packages.ros.org/ros/ubuntu $(lsb_release -sc) main" > /etc/apt/sources.list.d/ros-latest.list'

echo "[Download the ROS keys]"
sudo apt-key adv --keyserver 'hkp://keyserver.ubuntu.com:80' --recv-key C1CF6E31E6BADE8868B172B4F42ED6FBAB17C654

echo "[Update & upgrade the package]"
sudo apt-get update
#sudo apt-get upgrade -y

echo "[Installing ROS]"
sudo apt-get install -y ros-${name_ros_distro}-${package_type}

if ! [ $? -eq 0 ] ; then
  echo "Failure detected when installing ros packages, exiting"
  exit 1
fi

echo "[rosdep init and python-rosinstall]"
sudo sh -c "rosdep init"
rosdep update
. /opt/ros/$name_ros_distro/setup.sh
sudo apt-get install -y python-rosinstall

echo "[Making the catkin workspace and testing the catkin_make]"
mkdir -p ~/$name_catkinws/src
cd ~/$name_catkinws/src
catkin_init_workspace
cd ~/$name_catkinws/
catkin_make

echo "[Setting the ROS evironment]"
echo "source ~/${name_catkinws}/devel/setup.${user_shell}" | tee -a ~/.${user_shell}rc

echo "[Complete!!!]"

exit 0

