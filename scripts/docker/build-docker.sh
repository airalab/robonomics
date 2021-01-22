#!/usr/bin/env bash
set -e

pushd .

# The following line ensure we run from the project root
PROJECT_ROOT=`git rev-parse --show-toplevel`
FILE=$PROJECT_ROOT/target/release/robonomics
cd $PROJECT_ROOT/scripts/docker

# Find the current version from Cargo.toml
#VERSION=`grep "^version" ./Cargo.toml | egrep -o "([0-9\.]+)"`
GITUSER=robonomics
GITREPO=robonomics

# Copy robonomics binary if it's argument
[[ ! -z "$1" ]] && cp $1 .

# Build the image
echo "Building ${GITUSER}/${GITREPO}:latest docker image"
if [ ! -e robonomics ]
then
    # checks if file exist
    if [ ! -f "$FILE" ] 
    then
        echo "$FILE does not exist. You must have robonomics built first! to build go to project root direction and enter command: cargo build --release"
        exit 1
    fi
    # contine if file exists
    echo "If first docker build, copying file robonomics to working directory"
    cp ../../target/release/robonomics .
else
    echo "If not first build, proceed to docker-compose build and run"
fi
time docker build -f ./Dockerfile --build-arg RUSTC_WRAPPER= --build-arg PROFILE=release -t robonomics/robonomics:latest .

# cleanup binary file
rm robonomics

# Show the list of available images for this repo
echo "Image is ready"
docker images | grep ${GITREPO}

echo -e "\nIf you just built version ${VERSION}, you may want to update your tag:"
echo " $ docker tag ${GITUSER}/${GITREPO}:$VERSION ${GITUSER}/${GITREPO}:${VERSION}"

docker run -d -P --name robonomics robonomics/robonomics:latest

echo "Docker container is ready"
docker ps || grep ${GITREPO}

popd
