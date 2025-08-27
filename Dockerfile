FROM rust:latest AS builder

RUN apt-get update && apt-get install -y \
    build-essential pkg-config libssl-dev libpq-dev ffmpeg libavcodec-dev libavformat-dev libavutil-dev libswscale-dev libwayland-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

COPY api.Cargo.toml ./Cargo.toml

COPY Cargo.lock ./
COPY wayclip_api/Cargo.toml ./wayclip_api/
COPY wayclip_core/Cargo.toml ./wayclip_core/

RUN cargo fetch

COPY . .

RUN cargo build --release --package wayclip_api

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 libpq5 ffmpeg \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/wayclip_api /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/wayclip_api"]
