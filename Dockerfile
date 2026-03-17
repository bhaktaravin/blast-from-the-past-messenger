# Build stage
FROM rust:1.75 as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# Copy manifest files first (these change less frequently)
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to cache dependencies
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/bin/server.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release --features server 2>&1 | grep -v "warning:" || true

# Now copy the actual source code
COPY src ./src

# Build the actual binary
RUN cargo build --release --bin server --features server

# Runtime stage - minimal image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy only the binary from builder
COPY --from=builder /app/target/release/server ./server

# Expose the port
EXPOSE 9001

# Set the start command
CMD ["./server"]
