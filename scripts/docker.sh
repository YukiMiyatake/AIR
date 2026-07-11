#!/usr/bin/env bash
# Helpers for Docker-first AIR development.
set -euo pipefail
cd "$(dirname "$0")/.."

cmd="${1:-}"
shift || true

case "$cmd" in
  build)
    docker compose build dev
    ;;
  shell)
    docker compose run --rm dev bash
    ;;
  ts)
    docker compose run --rm dev npm run airc -- "$@"
    ;;
  rs)
    docker compose run --rm dev cargo run -p airc -- "$@"
    ;;
  test)
    docker compose run --rm dev npm test
    docker compose run --rm dev cargo test --workspace
    ;;
  *)
    echo "Usage: $0 {build|shell|ts|rs|test} [args...]"
    echo "  build  — build air-dev image"
    echo "  shell  — interactive bash in air-dev"
    echo "  ts     — npm run airc -- ..."
    echo "  rs     — cargo run -p airc -- ..."
    echo "  test   — TS + Rust tests in container"
    exit 2
    ;;
esac
