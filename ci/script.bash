#!/usr/bin/env bash

set -ex

# Incorporate TARGET env var to the build and test process
cargo build --target "$TARGET" --verbose

cargo test --target "$TARGET" --verbose

TEST_URI="https://github.com/mpv-player/mpv/archive/49d6a1e77d3dba5cf30a2cb9cd05dc6dd6407fb2.zip"
TEST_MD5="703cd17daf71138b90622d52fe6fb6a5"
TEST_NAME="49d6a1e77d3dba5cf30a2cb9cd05dc6dd6407fb2.zip"

RUST_BACKTRACE=1 cargo run --target "$TARGET" -- $TEST_URI

if [ "$TRAVIS_OS_NAME" = "linux" ]
then
    md5="$(cat $TEST_NAME | md5sum | cut -f1 -d' ')"
elif [ "$TRAVIS_OS_NAME" = "osx" ]
then
    md5="$(cat $TEST_NAME | md5 | cut -f1 -d' ')"
fi

if [ "$md5" != "$TEST_MD5" ]; then exit 1; fi

rm $TEST_NAME
