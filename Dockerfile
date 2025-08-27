FROM rust:latest AS builder

RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libpq-dev \
    ffmpeg \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libswscale-dev \
    libavfilter-dev \
    libwayland-dev \
    libxkbcommon-dev \
    libpipewire-0.3-dev \
    libdbus-1-dev \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libx11-dev \
    libxrandr-dev \
    libxtst-dev \
    libasound2-dev \
    clang \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app
ENV SQLX_OFFLINE=1

COPY api.Cargo.toml ./Cargo.toml

COPY Cargo.lock ./
COPY wayclip_api/Cargo.toml ./wayclip_api/
COPY wayclip_core/Cargo.toml ./wayclip_core/

RUN cargo fetch

COPY . .

RUN cargo build --release --package wayclip_api

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 \
    libpq5 \
    ffmpeg \
    libwayland-client0 \
    libxkbcommon0 \
    libpipewire-0.3-0 \
    libdbus-1-3 \
    libgstreamer1.0-0 \
    libgstreamer-plugins-base1.0-0 \
    libx11-6 \
    libxrandr2 \
    libxtst6 \
    libasound2 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/wayclip_api /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/wayclip_api"]
