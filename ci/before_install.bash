#!/usr/bin/env bash

set -ex

echo "-: before_install.bash"

if [[ $TRAVIS_OS_NAME != linux ]]; then
    exit 0
fi

echo "-: sudo apt-get update"
sudo apt-get update

# needed for musl targets
echo "-: sudo apt-get install -y musl-tools"
sudo apt-get install -y musl-tools

# needed to build deb packages
echo "-: sudo apt-get install -y fakeroot"
sudo apt-get install -y fakeroot

# needed for i686 linux gnu target
if [[ $TARGET == i686-unknown-linux-gnu ]]; then
    echo "-: sudo apt-get install -y gcc-multilib"
    sudo apt-get install -y gcc-multilib
fi
