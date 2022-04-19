#!/usr/bin/env bash

set -ex

TEST_AIM_URI="https://github.com/mpv-player/mpv/archive/49d6a1e77d3dba5cf30a2cb9cd05dc6dd6407fb2.zip"
TEST_AIM_MD5="703cd17daf71138b90622d52fe6fb6a5"
TEST_AIM_NAME="49d6a1e77d3dba5cf30a2cb9cd05dc6dd6407fb2.zip"

RUST_BACKTRACE=1 cargo run -- $TEST_AIM_URI

if [ "$RUNNER_OS" = "Linux" ]
then
    md5="$(cat $TEST_AIM_NAME | md5sum | cut -f1 -d' ')"
elif [ "$RUNNER_OS" = "macOS" ]
then
    md5="$(cat $TEST_AIM_NAME | md5 | cut -f1 -d' ')"
fi

if [ "$md5" != "$TEST_AIM_MD5" ]; then exit 1; fi

rm $TEST_AIM_NAME
