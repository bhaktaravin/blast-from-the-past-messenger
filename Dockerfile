# Build stage
FROM rust:1.75 as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Copy source code  
COPY src ./src

# Verify features are available
RUN echo "Building server binary with --features server" && \
    cargo build --release --bin server --features server && \
    ls -lh target/release/server

# Runtime stage - minimal image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy only the binary from builder
COPY --from=builder /app/target/release/server ./server

# Verify binary was copied
RUN ls -lh ./server

# Expose the port
EXPOSE 9001

# Set the start command
CMD ["./server"]
