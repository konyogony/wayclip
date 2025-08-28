# STAGE 1: Build the application
FROM --platform=$BUILDPLATFORM rust:latest AS builder

# Install the aarch64 target for cross-compilation
RUN rustup target add aarch64-unknown-linux-gnu

# Install build dependencies and the cross-compilation toolchain for ARM64
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    clang \
    # The C cross-compiler and linker for ARM64
    gcc-aarch64-linux-gnu \
    # --- FIX: Add binutils for the target architecture ---
    # This provides the assembler ('as') and other tools needed by GCC.
    binutils-aarch64-linux-gnu \
    # Development libraries
    libssl-dev libpq-dev libssh2-1-dev \
    ffmpeg libavcodec-dev libavformat-dev libavutil-dev libswscale-dev libavfilter-dev libavdevice-dev libswresample-dev \
    libwayland-dev libxkbcommon-dev libpipewire-0.3-dev libdbus-1-dev \
    libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
    libx11-dev libxrandr-dev libxtst-dev libasound2-dev \
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
RUN if [ "$TARGETPLATFORM" = "linux/arm64" ]; then \
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
