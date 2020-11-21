FROM rust:latest as dependencies

WORKDIR /app
RUN USER=root cargo new --bin dependencies
WORKDIR /app/dependencies
COPY samotop-server/Cargo.toml .
RUN cargo fetch && cat Cargo.lock
