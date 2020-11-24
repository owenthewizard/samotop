FROM rust:latest as builder

##########################################
# Download and install dev tools
##########################################

# install rust tools
RUN rustup component add clippy rustfmt
# install cargo tools
RUN cargo install toml-cli cargo-readme cargo-sweep
# install apt packages
RUN apt-get update && apt-get install -y \
   jq
# install wildq
RUN VERSION=$(curl -s "https://api.github.com/repos/ahmet2mir/wildq/releases/latest" | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/') \
    && curl -sL https://github.com/ahmet2mir/wildq/releases/download/v${VERSION}/wildq_${VERSION}-1_amd64.deb -o wildq_${VERSION}-1_amd64.deb \
    && dpkg -i wildq_${VERSION}-1_amd64.deb

FROM builder as dev

##########################################
# Download, build and cache dependencies
##########################################

RUN USER=root cargo new --lib app
WORKDIR /app
COPY samotop-model/Cargo.toml samotop-model/Cargo.toml
COPY samotop-core/Cargo.toml samotop-core/Cargo.toml
COPY samotop-parser/Cargo.toml samotop-parser/Cargo.toml
COPY samotop-delivery/Cargo.toml samotop-delivery/Cargo.toml
COPY samotop-with-spf/Cargo.toml samotop-with-spf/Cargo.toml
COPY samotop/Cargo.toml samotop/Cargo.toml
COPY samotop-server/Cargo.toml samotop-server/Cargo.toml

ENV LC_ALL=C.UTF-8
ENV LANG=C.UTF-8
RUN mv Cargo.toml Cargo.template.toml
RUN cat Cargo.template.toml | wildq -M -i toml -o toml 'del(.dependencies)' > Cargo.toml
RUN find . -name Cargo.toml -mindepth 1 \
    | xargs -I{} toml get {} dependencies \
    | jq -s 'add | to_entries | .[] |  select((.value|type=="string") or (.value.path?|not)) | del(.value.optional?)' \
    | jq -s 'from_entries' \
    | wildq -M -i json -o toml '{"dependencies": .}' | tee -a Cargo.toml
RUN find . -name Cargo.toml -mindepth 1 \
    | xargs -I{} toml get {} dev-dependencies \
    | jq -s 'add | to_entries | .[] |  select((.value|type=="string") or (.value.path?|not))' \
    | jq -s 'from_entries' \
    | wildq -M -i json -o toml '{"dev-dependencies": .}' | tee -a Cargo.toml
RUN cargo check && cargo build && cargo test --all-features

####################################
# The actual build of the app
####################################

COPY . .
RUN cargo check --color always --all-features \
    && echo "CLIPPY -------------------------------------------" \
    && cargo clippy --color always --all-features -- -Dclippy::all \
    && echo "BUILD -------------------------------------------" \
    && cargo build --color always --all-features \
    && echo "TEST -------------------------------------------" \
    && cargo test --color always --all-features \
    && echo "----- DEV DONE!"


FROM dev as prod
RUN cargo build --color always --release

####################################
# Samotop server build
####################################

FROM debian:buster-slim as server
COPY --from=prod /app/target/release/samotop-server /usr/local/bin/samotop
#COPY -Samotop.crt Samotop.crt
#COPY -Samotop.key Samotop.key
#COPY -Samotop.pfx Samotop.pfx
ENTRYPOINT ["samotop"]
CMD ["--help"]
USER 1001
