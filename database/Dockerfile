FROM rust:1.77 as builder

WORKDIR /app
COPY . .

RUN cargo build --release

FROM debian:bullseye-slim

WORKDIR /app

# Install required dependencies
RUN apt-get update && \
    apt-get install -y libssl-dev ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/charms-database /usr/local/bin/charms-database

EXPOSE 8080

CMD ["charms-database"]
