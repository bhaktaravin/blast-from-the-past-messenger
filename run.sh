#!/bin/bash
set -e

# Load .env
if [ ! -f .env ]; then
  echo "ERROR: .env file not found"
  exit 1
fi

set -a
source .env
set +a

if [ -z "$DATABASE_URL" ] || [ -z "$REDIS_URL" ]; then
  echo "ERROR: DATABASE_URL and REDIS_URL must be set in .env"
  exit 1
fi

echo "Building..."
cargo build --bin server --features server
cargo build --bin chatmessagediscordclone --features client

echo "Starting server..."
if nc -z localhost 9001 2>/dev/null; then
  echo "Server already running on port 9001, skipping..."
else
  DATABASE_URL="$DATABASE_URL" REDIS_URL="$REDIS_URL" BIND_ADDR="$BIND_ADDR" \
    ./target/debug/server &
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

echo "Starting client..."
./target/debug/chatmessagediscordclone

kill $SERVER_PID 2>/dev/null
