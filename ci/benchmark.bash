#!/usr/bin/env bash

set -ex

echo "-: benchmark.bash"

if [[ $TRAVIS_OS_NAME != linux ]]; then
    exit 0
fi


echo "-: install nginx"
sudo apt-get install -y nginx curl

echo "-: start nginx"
sudo nginx -c "$(pwd)/ci/benchmark.nginx.conf"

echo "-: make test file"
sudo mkdir /data
sudo dd if=/dev/zero of=file.txt count=10240 bs=1024
sudo mv file.txt /data/abc

echo "-: benchmark test begins"

echo "-: Request with one connection"
time curl http://localhost:9010/abc > abc
rm abc

echo "-: Request with 10 connections"
time target/debug/ag http://localhost:9010/abc -s 10 -k 1024
rm abc

echo "-: Request with 100 connections"
time target/debug/ag http://localhost:9010/abc -s 100 -k 102
rm abc

echo "-: benchmark test ends"
