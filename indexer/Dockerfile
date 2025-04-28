# Build stage: Compile Rust binaries
FROM rust:latest AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin charms-indexer
RUN cargo build --release --bin migrate

# Runtime stage: Lightweight image with binaries
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/charms-indexer /usr/local/bin/charms-indexer
COPY --from=builder /app/target/release/migrate /usr/local/bin/migrate
CMD ["charms-indexer"]
