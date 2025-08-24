FROM rust:1.78 as builder

WORKDIR /usr/src/app

COPY ./Cargo.toml ./Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

COPY ./src ./src
RUN cargo install --path .

FROM debian:buster-slim

COPY ./assets /usr/src/app/assets
COPY ./migrations /usr/src/app/migrations

COPY --from=builder /usr/local/cargo/bin/wayclip_api /usr/local/bin/wayclip_api

CMD ["wayclip_api"]
