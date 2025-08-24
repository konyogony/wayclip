FROM rust:latest as builder

WORKDIR /usr/src/app

COPY ./Cargo.toml ./Cargo.lock ./

RUN mkdir -p ./wayclip_api/src && echo "fn main() {}" > ./wayclip_api/src/main.rs
RUN mkdir -p ./wayclip_core/src && echo "fn main() {}" > ./wayclip_core/src/lib.rs 

RUN cargo build --release

COPY ./wayclip_api ./wayclip_api
COPY ./wayclip_core ./wayclip_core

RUN cargo build --release -p wayclip-api

FROM debian:buster-slim

COPY ./wayclip_api/assets /usr/src/app/assets
COPY ./wayclip_api/migrations /usr/src/app/migrations

COPY --from=builder /usr/src/app/target/release/wayclip-api /usr/local/bin/wayclip-api

CMD ["wayclip-api"]
