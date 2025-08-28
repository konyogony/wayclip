# STAGE 1: Build the application
FROM --platform=$BUILDPLATFORM rust:latest AS builder

# Enable the arm64 architecture in Debian's package manager
RUN dpkg --add-architecture arm64

# Install the aarch64 target for Rust
RUN rustup target add aarch64-unknown-linux-gnu

# Install build dependencies, the cross-compilation toolchain,
# and the development libraries FOR THE ARM64 TARGET.
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Native build tools for the amd64 host
    build-essential \
    pkg-config \
    clang \
    # Cross-compilation toolchain
    gcc-aarch64-linux-gnu \
    binutils-aarch64-linux-gnu \
    libc6-dev-arm64-cross \
    # Development packages for the arm64 target architecture
    libssl-dev:arm64 \
    libpq-dev:arm64 \
    libssh2-1-dev:arm64 \
    libavcodec-dev:arm64 libavformat-dev:arm64 libavutil-dev:arm64 libswscale-dev:arm64 libavfilter-dev:arm64 libavdevice-dev:arm64 libswresample-dev:arm64 \
    libwayland-dev:arm64 \
    libxkbcommon-dev:arm64 \
    libpipewire-0.3-dev:arm64 \
    libdbus-1-dev:arm64 \
    libgstreamer1.0-dev:arm64 \
    libgstreamer-plugins-base1.0-dev:arm64 \
    libx11-dev:arm64 \
    libxrandr-dev:arm64 \
    libxtst-dev:arm64 \
    libasound2-dev:arm64 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# This configuration tells Cargo to use the ARM64 GCC toolchain as the linker
# when building for the 'aarch64-unknown-linux-gnu' target.
RUN mkdir -p .cargo && \
    echo '[target.aarch64-unknown-linux-gnu]' >> .cargo/config.toml && \
    echo 'linker = "aarch64-linux-gnu-gcc"' >> .cargo/config.toml

# --- Dependency Caching Layer ---
COPY api.Cargo.toml ./Cargo.toml
COPY Cargo.lock ./
COPY wayclip_api/Cargo.toml ./wayclip_api/
COPY wayclip_core/Cargo.toml ./wayclip_core/
RUN cargo fetch

# --- Build Layer ---
COPY . .

# Build the application, checking the TARGETPLATFORM variable
ARG TARGETPLATFORM
# --- FIX: Explicitly set OpenSSL env vars to bypass pkg-config issues ---
RUN if [ "$TARGETPLATFORM" = "linux/arm64" ]; then \
    export PKG_CONFIG_PATH="/usr/lib/aarch64-linux-gnu/pkgconfig" && \
    export OPENSSL_LIB_DIR="/usr/lib/aarch64-linux-gnu" && \
    export OPENSSL_INCLUDE_DIR="/usr/include/aarch64-linux-gnu" && \
    cargo build --release --package wayclip_api --target aarch64-unknown-linux-gnu; \
    else \
    cargo build --release --package wayclip_api; \
    fi

# STAGE 2: Create the final, lean production image
FROM debian:bookworm-slim

# Install only the necessary runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 libpq5 libssh2-1 ffmpeg libwayland-client0 libxkbcommon0 \
    libpipewire-0.3-0 libdbus-1-3 libgstreamer1.0-0 libgstreamer-plugins-base1.0-0 \
    libx11-6 libxrandr2 libxtst6 libasound2 \
    && rm -rf /var/lib/apt/lists/*

# Correctly copy the compiled binary from the builder stage
ARG TARGETPLATFORM
COPY --from=builder /usr/src/app/target/$(if [ "$TARGETPLATFORM" = "linux/arm64" ]; then echo "aarch64-unknown-linux-gnu/release"; else echo "release"; fi)/wayclip_api /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/wayclip_api"]
