# STAGE 1: Build the application with ALL possible desktop dependencies
FROM rust:latest AS builder

# Install a comprehensive list of development libraries to satisfy the entire dependency tree of wayclip_core.
RUN apt-get update && apt-get install -y \
    # Core build tools
    build-essential \
    pkg-config \
    # For reqwest (openssl) and sqlx (postgres)
    libssl-dev \
    libpq-dev \
    # For ffmpeg-next
    ffmpeg \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libswscale-dev \
    # For wayland-client, ashpd, xcap
    libwayland-dev \
    libxkbcommon-dev \
    # For ashpd (pipewire support)
    libpipewire-0.3-dev \
    # For ashpd (dbus portals)
    libdbus-1-dev \
    # For gstreamer
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    # For xcap (X11 screen capture)
    libx11-dev \
    libxrandr-dev \
    libxtst-dev \
    # For rodio (audio)
    libasound2-dev \
    # --- THE FINAL MISSING PIECE ---
    # For bindgen, which needs to parse C header files
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Use the api-specific workspace file
COPY api.Cargo.toml ./Cargo.toml

COPY Cargo.lock ./
COPY wayclip_api/Cargo.toml ./wayclip_api/
COPY wayclip_core/Cargo.toml ./wayclip_core/

# This will fetch all Rust dependencies
RUN cargo fetch

# Copy the actual source code
COPY . .

# Build the API binary
RUN cargo build --release --package wayclip_api

# STAGE 2: Create the final image with all the RUNTIME libraries
FROM debian:bookworm-slim

# Install the non-dev versions of all the libraries needed by the final binary
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

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/app/target/release/wayclip_api /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/wayclip_api"]
