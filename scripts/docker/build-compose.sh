#!/bin/bash

# The following line ensure we run from the project root
PROJECT_ROOT=`git rev-parse --show-toplevel`
FILE=$PROJECT_ROOT/target/release/robonomics

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
# load docker compose in detached mode
docker-compose up -d --build 

# cleanup binary file
rm robonomics

echo "Docker-compose has been completed, to check status, type: docker ps"
