#!/usr/bin/env sh

set -ex

cargo +nightly fmt
cargo +nightly clippy