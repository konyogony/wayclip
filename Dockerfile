FROM rust:latest as builder

WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock ./

RUN mkdir -p ./wayclip_api ./wayclip_cli ./wayclip_core "wayclip_gui/src-tauri"

COPY wayclip_api/Cargo.toml ./wayclip_api/
COPY wayclip_cli/Cargo.toml ./wayclip_cli/
COPY wayclip_core/Cargo.toml ./wayclip_core/
COPY wayclip_gui/src-tauri/Cargo.toml ./wayclip_gui/src-tauri/

RUN mkdir -p ./wayclip_api/src && echo "fn main() {}" > ./wayclip_api/src/main.rs
RUN mkdir -p ./wayclip_cli/src && echo "fn main() {}" > ./wayclip_cli/src/main.rs
RUN mkdir -p ./wayclip_core/src && echo "fn main() {}" > ./wayclip_core/src/lib.rs
RUN mkdir -p ./wayclip_gui/src-tauri/src && echo "fn main() {}" > ./wayclip_gui/src-tauri/src/main.rs

RUN cargo build --release

COPY ./wayclip_api/src ./wayclip_api/src
COPY ./wayclip_cli/src ./wayclip_cli/src
COPY ./wayclip_core/src ./wayclip_core/src
COPY ./wayclip_gui/src-tauri/src ./wayclip_gui/src-tauri/src

RUN cargo build --release -p wayclip-api

FROM debian:buster-slim

COPY ./wayclip_api/assets /usr/src/app/assets
COPY ./wayclip_api/migrations /usr/src/app/migrations

COPY --from=builder /usr/src/app/target/release/wayclip-api /usr/local/bin/wayclip-api

CMD ["wayclip-api"]
