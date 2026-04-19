#!/bin/bash
set -e

# Load .env file
if [ -f .env ]; then
  echo "Loading environment variables from .env..."
  set -a
  source .env
  set +a
else
  echo "Error: .env file not found!"
  exit 1
fi

echo "Starting server..."
cargo run --bin server --features server
