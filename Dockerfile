# Stage 1: Build the Rust binary
FROM rust:1.92-trixie as builder

WORKDIR /app

# Copy workspace configuration
COPY Cargo.toml Cargo.lock ./
COPY deny.toml ./
COPY diesel.toml ./

# Copy all crates
COPY crates ./crates
COPY xtask ./xtask

# Build the server binary in release mode
RUN cargo build --release

# Stage 2: Create minimal runtime image
FROM debian:trixie-20260112-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libmariadb3 \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 zabbid

# Copy binary from builder
COPY --from=builder /app/target/release/zab-bid-server /usr/local/bin/zab-bid-server

# Set ownership
RUN chown zabbid:zabbid /usr/local/bin/zab-bid-server

# Switch to non-root user
USER zabbid

# Expose port
EXPOSE 8080

# Run the server
CMD zab-bid-server \
    --db-backend mysql \
    --database-url "$DATABASE_URL"
