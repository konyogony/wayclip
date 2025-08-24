FROM rust:latest as builder

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./
COPY wayclip_api/Cargo.toml ./wayclip_api/
COPY wayclip_cli/Cargo.toml ./wayclip_cli/
COPY wayclip_core/Cargo.toml ./wayclip_core/
COPY wayclip_gui/src-tauri/Cargo.toml ./wayclip_gui/src-tauri/

COPY .sqlx ./.sqlx

RUN mkdir -p ./wayclip_api/src && echo "fn main() {}" > ./wayclip_api/src/main.rs
RUN mkdir -p ./wayclip_cli/src && echo "fn main() {}" > ./wayclip_cli/src/main.rs
RUN mkdir -p ./wayclip_core/src && echo "pub fn lib() {}" > ./wayclip_core/src/lib.rs
RUN mkdir -p ./wayclip_gui/src-tauri/src && echo "pub fn lib() {}" > ./wayclip_gui/src-tauri/src/lib.rs
RUN cargo build --release

COPY ./wayclip_api ./wayclip_api
COPY ./wayclip_cli ./wayclip_cli
COPY ./wayclip_core ./wayclip_core
COPY ./wayclip_gui/src-tauri ./wayclip_gui/src-tauri

RUN cargo build --release -p wayclip_api

FROM debian:buster-slim

WORKDIR /usr/src/app

COPY ./wayclip_api/assets ./assets
COPY ./wayclip_api/migrations ./migrations

COPY --from=builder /usr/src/app/target/release/wayclip_api /usr/local/bin/wayclip_api

CMD ["wayclip_api"]
