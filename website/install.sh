#!/bin/sh
set -eu

status() {
    printf "\r\033[K%s" "$1"
}

if ! command -v cargo >/dev/null 2>&1; then
    printf "ERROR: 'cargo' is required but not installed\n"
    printf "       Install Rust from https://rustup.rs\n"
    exit 1
fi

status "Installing SupGIT via cargo..."
cargo install supgit

printf "\r\033[K🎉 SupGIT is installed 🎉\n"
