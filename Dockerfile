# Stage 1: build
FROM rust:latest AS builder
RUN apt-get update && apt-get install -y \
    build-essential pkg-config libssl-dev libpq-dev libssh2-1-dev ffmpeg libavcodec-dev libavformat-dev libavutil-dev libswscale-dev libwayland-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./
COPY wayclip_cli ./wayclip_cli/
COPY wayclip_core ./wayclip_core/

# Only build dependencies first for caching
RUN cargo build --release --manifest-path ./wayclip_cli/Cargo.toml || true

# Copy source code after caching deps
COPY . .

RUN cargo build --release --manifest-path ./wayclip_cli/Cargo.toml
