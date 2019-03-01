#!/usr/bin/env bash

set -ex

echo "-: before_install.bash"

if [[ $TRAVIS_OS_NAME != linux ]]; then
    exit 0
fi

echo "-: sudo apt-get update"
sudo apt-get update

# needed for aget-rs
sudo apt-get install -y build-essential openssl libssl-dev pkg-config

# needed to build deb packages
echo "-: sudo apt-get install -y fakeroot"
sudo apt-get install -y fakeroot
