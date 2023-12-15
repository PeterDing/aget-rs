#!/usr/bin/env bash

if ! command -V sudo; then
  apt-get update
  apt-get install -y --no-install-recommends sudo
fi
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  zsh xz-utils liblz4-tool musl-tools brotli zstd \
  build-essential openssl libssl-dev pkg-config

# needed to build deb packages
sudo apt-get install -y --no-install-recommends fakeroot
