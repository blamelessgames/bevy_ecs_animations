#/usr/bin/env sh

set -e

cargo fmt --all -- --check

cargo clippy --workspace --all-targets --all-features -- -D warnings

cargo build --workspace --all-targets --all-features

cargo test --workspace --all-targets --all-features