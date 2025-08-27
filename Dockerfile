FROM rust:latest AS builder

RUN apt-get update && apt-get install -y \
    build-essential pkg-config libssl-dev libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./
COPY wayclip_api/Cargo.toml ./wayclip_api/
COPY wayclip_core/Cargo.toml ./wayclip_core/

RUN cargo check -p wayclip_api

COPY . .

RUN cargo build --release --manifest-path ./wayclip_api/Cargo.toml

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 libpq5 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/wayclip_api /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/wayclip_api"]
