#!/bin/bash
if [ ! -e robonomics ]
then
    echo "If first docker build, copying file robonomics to working directory"
cp /root/robonomics/target/release/robonomics .
else
    echo "If not first build, proceed to docker-compose build and run"
fi
# load docker compose in detached mode
docker-compose up -d

echo "Docker-compose has been completed, to check status, type: docker ps"
