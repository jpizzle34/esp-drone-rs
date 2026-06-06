#!/usr/bin/env bash
# Stable rust-analyzer for IDE use. The esp toolchain does not ship rust-analyzer.
exec "${HOME}/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rust-analyzer" "$@"
