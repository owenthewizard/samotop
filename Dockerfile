##########################################
# Download and install dev tools
##########################################
FROM debian:buster-slim as builder
ENV LC_ALL=C.UTF-8
ENV LANG=C.UTF-8

LABEL maintainer="Jo Cutajar <tell-no-one@robajz.info>"

RUN cat /etc/apt/sources.list | sed 's/^deb /deb-src /g' > /etc/apt/sources.list.d/src.list

# Required packages:
# - build-essential, musl-dev, musl-tools - the C/C++/libc toolchain
# - curl + ca-certificates - for fetching and verifying online resources
# - libssl-dev - for dynamic linking openssl
RUN apt-get update && apt-get upgrade -y && apt-get install -y \
    musl-dev \
    musl-tools \
    build-essential \
    ca-certificates \
    curl \
    jq \
    libssl-dev \
    --no-install-recommends
    # would break apt-get source
    #rm -rf /var/lib/apt/lists/*

# Essential compilation vars
# SSL cert directories get overridden by --prefix and --openssldir
# and they do not match the typical host configurations.
# The SSL_CERT_* vars fix this, but only when inside this container
# musl-compiled binary must point SSL at the correct certs (muslrust/issues/5) elsewhere
ENV CC=musl-gcc \
    PREFIX=/musl \
    PATH=$PREFIX/bin:/usr/local/bin:/root/.cargo/bin:$PATH \
    PKG_CONFIG_PATH=/usr/local/lib/pkgconfig \
    LD_LIBRARY_PATH=$PREFIX \
    PKG_CONFIG_ALLOW_CROSS=true \
    PKG_CONFIG_ALL_STATIC=true \
    PKG_CONFIG_PATH=$PREFIX/lib/pkgconfig \
    OPENSSL_STATIC=true \
    OPENSSL_DIR=$PREFIX \
    SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt \
    SSL_CERT_DIR=/etc/ssl/certs

# Set up a prefix for musl build libraries, make the linker's job of finding them easier
# Primarily for the benefit of postgres.
# Lastly, link some linux-headers for openssl 1.1 (not used herein)
RUN mkdir $PREFIX && \
    echo "$PREFIX/lib" >> /etc/ld-musl-x86_64.path && \
    ln -s /usr/include/x86_64-linux-gnu/asm /usr/include/x86_64-linux-musl/asm && \
    ln -s /usr/include/asm-generic /usr/include/x86_64-linux-musl/asm-generic && \
    ln -s /usr/include/linux /usr/include/x86_64-linux-musl/linux

# Download and build official debian openssl package but with musl
RUN apt-get source openssl && \
    cd openssl* && \
    ./Configure no-shared -fPIC --prefix=$PREFIX --openssldir=$PREFIX/ssl linux-x86_64 && \
    env C_INCLUDE_PATH=$PREFIX/include make depend 2> /dev/null && \
    make -j$(nproc) && make install_sw && \
    cd .. && rm -rf openssl*

# Rustup stable
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    --default-toolchain stable \
    --profile minimal \
    --component rustfmt \
    --component clippy \
    --target x86_64-unknown-linux-musl \
    --target x86_64-unknown-linux-gnu

# Useful cargo tools
RUN cargo install toml-cli cargo-readme cargo-sweep cargo-audit cargo-outdated

# Nightly rust
RUN rustup toolchain install nightly \
    --profile minimal \
    --component rustfmt \
    --component clippy \
    --target x86_64-unknown-linux-musl \
    --target x86_64-unknown-linux-gnu

##########################################
# Download, build and cache dependencies
##########################################
FROM builder as deps

# install wildq
RUN WQ_VERSION=$(curl -s "https://api.github.com/repos/ahmet2mir/wildq/releases/latest" | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/') \
    && curl -sL https://github.com/ahmet2mir/wildq/releases/download/v${WQ_VERSION}/wildq_${WQ_VERSION}-1_amd64.deb -o wildq_${WQ_VERSION}-1_amd64.deb \
    && dpkg -i wildq_${WQ_VERSION}-1_amd64.deb \
    && rm wildq_${WQ_VERSION}-1_amd64.deb

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
