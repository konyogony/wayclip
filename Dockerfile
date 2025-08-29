FROM --platform=$BUILDPLATFORM rust:latest AS builder

ARG TARGETPLATFORM

RUN apt-get update && \
    export DEBIAN_FRONTEND=noninteractive && \
    if [ "$TARGETPLATFORM" = "linux/arm64" ]; then \
        dpkg --add-architecture arm64 && \
        apt-get update && \
        apt-get install -y --no-install-recommends \
            build-essential pkg-config clang \
            gcc-aarch64-linux-gnu binutils-aarch64-linux-gnu libc6-dev-arm64-cross \
            libssl-dev:arm64 libpq-dev:arm64 libssh2-1-dev:arm64 \
            libavcodec-dev:arm64 libavformat-dev:arm64 libavutil-dev:arm64 libswscale-dev:arm64 libavfilter-dev:arm64 libavdevice-dev:arm64 libswresample-dev:arm64 \
            libwayland-dev:arm64 libxkbcommon-dev:arm64 libpipewire-0.3-dev:arm64 libdbus-1-dev:arm64 \
            libgstreamer1.0-dev:arm64 libgstreamer-plugins-base1.0-dev:arm64 \
            libx11-dev:arm64 libxrandr-dev:arm64 libxtst-dev:arm64 libasound2-dev:arm64; \
    else \
        apt-get install -y --no-install-recommends \
            build-essential pkg-config clang \
            libssl-dev libpq-dev libssh2-1-dev \
            libavcodec-dev libavformat-dev libavutil-dev libswscale-dev libavfilter-dev libavdevice-dev libswresample-dev \
            libwayland-dev libxkbcommon-dev libpipewire-0.3-dev libdbus-1-dev \
            libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
            libx11-dev libxrandr-dev libxtst-dev libasound2-dev; \
    fi && \
    rm -rf /var/lib/apt/lists/*

RUN rustup target add aarch64-unknown-linux-gnu

WORKDIR /usr/src/app

RUN mkdir -p .cargo && \
    echo '[target.aarch64-unknown-linux-gnu]' >> .cargo/config.toml && \
    echo 'linker = "aarch64-linux-gnu-gcc"' >> .cargo/config.toml

COPY api.Cargo.toml ./Cargo.toml
COPY Cargo.lock ./
COPY wayclip_api/Cargo.toml ./wayclip_api/
COPY wayclip_core/Cargo.toml ./wayclip_core/
RUN cargo fetch

COPY . .

RUN if [ "$TARGETPLATFORM" = "linux/arm64" ]; then \
    PKG_CONFIG="aarch64-linux-gnu-pkg-config" \
    cargo build --release --package wayclip_api --target aarch64-unknown-linux-gnu; \
    else \
    cargo build --release --package wayclip_api; \
    fi

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 libpq5 libssh2-1 ffmpeg libwayland-client0 libxkbcommon0 \
    libpipewire-0.3-0 libdbus-1-3 libgstreamer1.0-0 libgstreamer-plugins-base1.0-0 \
    libx11-6 libxrandr2 libxtst6 libasound2 \
    && rm -rf /var/lib/apt/lists/*

ARG TARGETPLATFORM
COPY --from=builder /usr/src/app/target/$(if [ "$TARGETPLATFORM" = "linux/arm64" ]; then echo "aarch64-unknown-linux-gnu/release"; else echo "release"; fi)/wayclip_api /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/wayclip_api"]
