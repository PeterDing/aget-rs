#!/usr/bin/env bash

set -ex

# Incorporate TARGET env var to the build and test process
cargo build --target "$TARGET" --verbose

cargo test --target "$TARGET" --verbose

RUST_BACKTRACE=1 cargo run --target "$TARGET" -- http://cdimage.ubuntu.com/ubuntu-base/releases/18.04/release/ubuntu-base-18.04-base-amd64.tar.gz

if [ "$TRAVIS_OS_NAME" = "linux" ]
then
    if [ "$(cat ubuntu-base-18.04-base-amd64.tar.gz | md5sum | cut -f1 -d' ')" != "2ed358ddea75aaa2aae924f2a4bc0aa0" ]; then exit 1; fi
elif [ "$TRAVIS_OS_NAME" = "osx" ]
then
    if [ "$(cat ubuntu-base-18.04-base-amd64.tar.gz | md5 | cut -f1 -d' ')" != "2ed358ddea75aaa2aae924f2a4bc0aa0" ]; then exit 1; fi
fi

rm ubuntu-base-18.04-base-amd64.tar.gz
