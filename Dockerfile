FROM rust:latest as dependencies

WORKDIR /app
RUN USER=root cargo new --bin dependencies
WORKDIR /app/dependencies
COPY Cargo.lock .
RUN cargo fetch