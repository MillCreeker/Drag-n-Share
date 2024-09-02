FROM rust:1.80.1 as build

# create a new empty shell project
RUN USER=root cargo new --bin api
WORKDIR /api

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN rustup target add x86_64-unknown-linux-musl

# copy your source tree
COPY ./src ./src

# build for release
RUN cargo build --bins --release --target x86_64-unknown-linux-musl


# smaller image
FROM alpine:3.20.2

WORKDIR /app

# copy the build artifact from the build stage
COPY --from=build /api/target/x86_64-unknown-linux-musl/release/api /app/api
COPY .env /app/.env

# set the startup command to run your binary
ENTRYPOINT ["/app/api"]