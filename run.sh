#!/bin/bash
set -e

# Load .env if present (only needed for running a local server)
if [ -f .env ]; then
  set -a
  source .env
  set +a
fi

echo "Building client..."
cargo build --bin chatmessagediscordclone --features client

# Only start a local server if DATABASE_URL and REDIS_URL are set
if [ -n "$DATABASE_URL" ] && [ -n "$REDIS_URL" ]; then
  if nc -z localhost 9001 2>/dev/null; then
    echo "Server already running on port 9001, skipping..."
  else
    echo "Starting local server..."
    cargo build --bin server --features server
    BIND_ADDR="${BIND_ADDR:-0.0.0.0:9001}" ./target/debug/server &
    SERVER_PID=$!

    echo "Waiting for server on port 9001..."
    for i in $(seq 1 30); do
      if nc -z localhost 9001 2>/dev/null; then
        echo "Server ready!"
        break
      fi
      sleep 0.5
    done
  fi
else
  echo "No .env found — connecting to Railway server..."
fi

echo "Starting client..."
./target/debug/chatmessagediscordclone

[ -n "$SERVER_PID" ] && kill $SERVER_PID 2>/dev/null