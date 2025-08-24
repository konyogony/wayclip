FROM rust:latest as builder

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./

RUN mkdir ./wayclip_api && mkdir ./wayclip_core
COPY wayclip_api/Cargo.toml ./wayclip_api/
COPY wayclip_core/Cargo.toml ./wayclip_core/


RUN mkdir -p ./wayclip_api/src && echo "fn main() {}" > ./wayclip_api/src/main.rs
RUN mkdir -p ./wayclip_core/src && echo "fn main() {}" > ./wayclip_core/src/lib.rs

RUN cargo build --release
COPY ./wayclip_api/src ./wayclip_api/src
COPY ./wayclip_core/src ./wayclip_core/src

RUN cargo build --release -p wayclip-api

FROM debian:buster-slim

COPY ./wayclip_api/assets /usr/src/app/assets
COPY ./wayclip_api/migrations /usr/src/app/migrations

COPY --from=builder /usr/src/app/target/release/wayclip-api /usr/local/bin/wayclip-api

CMD ["wayclip-api"]
