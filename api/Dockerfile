FROM rust:1.80.1 AS build

RUN apt-get update && apt-get install -y musl-tools musl-dev

# new empty shell project
RUN USER=root cargo new --bin api
WORKDIR /api

# copy manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# cache dependencies
RUN rustup target add x86_64-unknown-linux-musl

# copy source tree
COPY ./src ./src

# build for release
RUN cargo build --bins --release --target x86_64-unknown-linux-musl


# smaller image
FROM alpine:3.20.2

WORKDIR /app

# copy build artifact from build stage
COPY --from=build /api/target/x86_64-unknown-linux-musl/release/api /app/api
COPY .env /app/.env


ENTRYPOINT ["/app/api"]