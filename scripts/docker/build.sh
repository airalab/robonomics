#!/usr/bin/env bash
set -e

pushd .

# The following line ensure we run from the project root
PROJECT_ROOT=`git rev-parse --show-toplevel`
cd $PROJECT_ROOT/scripts/docker

# Find the current version from Cargo.toml
#VERSION=`grep "^version" ./Cargo.toml | egrep -o "([0-9\.]+)"`
GITUSER=robonomics
GITREPO=robonomics

# Copy robonomics binary if it's argument
[[ ! -z "$1" ]] && cp $1 .

# Build the image
echo "Building ${GITUSER}/${GITREPO}:latest docker image, hang on!"
time docker build -f ./Dockerfile --build-arg RUSTC_WRAPPER= --build-arg PROFILE=release -t robonomics/robonomics:latest .

# Show the list of available images for this repo
echo "Image is ready"
docker images | grep ${GITREPO}

echo -e "\nIf you just built version ${VERSION}, you may want to update your tag:"
echo " $ docker tag ${GITUSER}/${GITREPO}:$VERSION ${GITUSER}/${GITREPO}:${VERSION}"

popd
