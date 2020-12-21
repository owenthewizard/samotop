##########################################
# Download and install dev tools
##########################################
FROM clux/muslrust:latest as builder
ENV LC_ALL=C.UTF-8
ENV LANG=C.UTF-8

# Remove muslrust config to default to musl target
#RUN rm ~/.cargo/config

# install rust tools for stable
RUN rustup toolchain install stable
RUN rustup +stable component add clippy rustfmt
RUN rustup +stable target add x86_64-unknown-linux-musl
# install cargo tools
RUN cargo +stable install toml-cli cargo-readme cargo-sweep cargo-audit cargo-outdated
# install apt packages
RUN apt-get update \
    && apt-get upgrade -y \
    && apt-get install -y \
                    jq \
                    musl musl-dev musl-tools \
    && apt-get clean autoclean 
# install wildq
RUN VERSION=$(curl -s "https://api.github.com/repos/ahmet2mir/wildq/releases/latest" | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/') \
    && curl -sL https://github.com/ahmet2mir/wildq/releases/download/v${VERSION}/wildq_${VERSION}-1_amd64.deb -o wildq_${VERSION}-1_amd64.deb \
    && dpkg -i wildq_${VERSION}-1_amd64.deb \
    && rm wildq_${VERSION}-1_amd64.deb

##########################################
# Download, build and cache dependencies
##########################################
FROM builder as deps
RUN USER=root cargo new --lib /app
WORKDIR /app
COPY samotop/Cargo.toml samotop/Cargo.toml
COPY samotop-core/Cargo.toml samotop-core/Cargo.toml
COPY samotop-delivery/Cargo.toml samotop-delivery/Cargo.toml
COPY samotop-parser/Cargo.toml samotop-parser/Cargo.toml
COPY samotop-parser-nom/Cargo.toml samotop-parser-nom/Cargo.toml
COPY samotop-server/Cargo.toml samotop-server/Cargo.toml
COPY samotop-smime/Cargo.toml samotop-smime/Cargo.toml
COPY samotop-with-native-tls/Cargo.toml samotop-with-native-tls/Cargo.toml
COPY samotop-with-rustls/Cargo.toml samotop-with-rustls/Cargo.toml
COPY samotop-with-spf/Cargo.toml samotop-with-spf/Cargo.toml

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

# # The actual build of the app
# FROM deps as nightly
# COPY . .
# RUN cargo check --color always --all-features \
#     && echo "CLIPPY -------------------------------------------" \
#     && cargo clippy --color always --all-features -- -Dclippy::all \
#     && echo "BUILD -------------------------------------------" \
#     && cargo build --color always --all-features \
#     && echo "TEST -------------------------------------------" \
#     && cargo test --color always --all-features \
#     && echo "----- DEV DONE!"

# The actual build of the app
FROM deps as stable
COPY . .
RUN cargo +stable check --color always --all-features \
    && echo "CLIPPY -------------------------------------------" \
    && cargo +stable clippy --color always --all-features -- -Dclippy::all \
    && echo "BUILD -------------------------------------------" \
    && cargo +stable build --color always --all-features \
    && echo "TEST -------------------------------------------" \
    && cargo +stable test --color always --all-features \
    && echo "RELEASE ------------------------------------------" \
    && cargo +stable build --color always --release

####################################
# Samotop server build
####################################
FROM scratch as server
COPY --from=stable /app/target/*/release/samotop-server /bin/samotop
#COPY -Samotop.crt Samotop.crt
#COPY -Samotop.key Samotop.key
#COPY -Samotop.pfx Samotop.pfx
ENTRYPOINT ["/bin/samotop"]
CMD ["--help"]
USER 1001
