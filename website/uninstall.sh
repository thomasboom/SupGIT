#!/bin/sh
set -eu

status() {
    printf "\r\033[K%s" "$1"
}

usage() {
    printf "Usage: %s [--sgit]\n" "$0"
}

if [ "${1:-}" = "--sgit" ]; then
    shift

    if [ "$#" -ne 0 ]; then
        usage
        exit 1
    fi

    if ! command -v sgit >/dev/null 2>&1; then
        printf "ERROR: 'sgit' was not found in PATH\n"
        exit 1
    fi

    sgit_path=$(command -v sgit)
    status "Removing sgit binary from PATH..."
    rm -f "$sgit_path"
    printf "\r\033[K👋 sgit binary removed from PATH (%s) 👋\n" "$sgit_path"
    exit 0
fi

if [ "$#" -ne 0 ]; then
    usage
    exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
    printf "ERROR: 'cargo' is required but not installed\n"
    exit 1
fi

status "Uninstalling SupGIT..."
cargo uninstall supgit

printf "\r\033[K👋 SupGIT has been uninstalled 👋\n"
