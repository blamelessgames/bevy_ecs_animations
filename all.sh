#/usr/bin/env sh

set -e

cargo fmt --all -- --check

cargo clippy --all-targets --all-features -- -D warnings

cargo build --all-targets --all-features

cargo test --all-targets --all-features