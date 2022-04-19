#!/usr/bin/env bash

set -ex

echo "-: benchmark.bash"

if [[ $RUNNER_OS != Linux ]]; then
    exit 0
fi


echo "-: install nginx"
sudo apt-get install -y nginx curl

echo "-: start nginx"
sudo nginx -c "$(pwd)/ci/benchmark.nginx.conf"

echo "-: make test file"
sudo mkdir -p /data
# size: 10m
sudo dd if=/dev/zero of=file.txt count=10240 bs=1024
sudo mv file.txt /data/abc

echo "-: benchmark test begins"

echo "-: Request with one connection"
# 10240k / 100k/s = 102s = 1m42s
time curl http://localhost:9010/abc > abc
rm abc

echo "-: Request with 10 connections"
# 10240k / (10 * 100k/s) = 10s, each interval is 1m
time target/debug/ag http://localhost:9010/abc -s 10 -k 1m
rm abc

echo "-: Request with 100 connections"
# 10240k / (100 * 100k/s) = 1s, each interval is 102.4k
time target/debug/ag http://localhost:9010/abc -s 100 -k 103k
rm abc

echo "-: benchmark test ends"
