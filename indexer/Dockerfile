# Build stage: Compile Rust binaries.
# Pin to bookworm so the builder's glibc matches debian:bookworm-slim's
# glibc (2.36). rust:latest rolls forward to trixie (glibc 2.39+) and the
# resulting binary crashes at runtime with "version `GLIBC_2.38' not found".
FROM rust:bookworm AS builder
WORKDIR /app
COPY . .
# --locked forces use of Cargo.lock without re-resolving. Needed because the
# upstream charms-client workspace pins `bitcoin = "^0.32.100"` (yanked on
# crates.io) and a fresh resolve in Docker would fail. Cargo.lock has the
# exact commit and version, which is still installable.
RUN cargo build --release --bin charms-indexer --locked

# Runtime stage: Lightweight image with binaries
FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y libpq5 ca-certificates curl bash && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/charms-indexer /usr/local/bin/charms-indexer
CMD ["charms-indexer"]