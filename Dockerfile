##########################################
# Download and install dev tools
##########################################
FROM rust:1-slim as builder
LABEL maintainer="Jo Cutajar <tell-no-one@robajz.info>"
ENV LC_ALL=C.UTF-8
ENV LANG=C.UTF-8

RUN cat /etc/apt/sources.list | sed 's/^deb /deb-src /g' > /etc/apt/sources.list.d/src.list

# Required packages:
# - build-essential, musl-dev, musl-tools - the C/C++/libc toolchain
# - jq - for manipulating json as part of build
# - git - checking file modifications after rustfmt/readme in CI
# - curl - for CI tests
# - libssl-dev - required by some cargo tools (audit)
# - pkg-config - so that rust build scripts find the right lib headers (openssl)
RUN apt-get update && apt-get upgrade -y && apt-get install -y --no-install-recommends \
    musl-dev musl-tools build-essential \
    jq \
    git \
    curl \
    libssl-dev \
    pkg-config
    # would break apt-get source
    #rm -rf /var/lib/apt/lists/*

# Useful cargo tools
RUN cargo install toml-cli cargo-readme cargo-sweep cargo-audit cargo-outdated

# Rustup stable
RUN rustup --version --verbose
RUN rustup component add clippy
RUN rustup component add rustfmt
RUN rustup target add x86_64-unknown-linux-musl

# Nightly rust
RUN rustup toolchain install nightly \
    --profile minimal \
    --component rustfmt \
    --component clippy \
    --target x86_64-unknown-linux-musl \
    --target x86_64-unknown-linux-gnu

# Where MUSL builds will be prefixed
ENV MUSLPREFIX=/var/local/musl
ENV PKG_CONFIG_ALLOW_CROSS=true \
    PKG_CONFIG_ALL_STATIC=true \
    PKG_CONFIG_PATH="$MUSLPREFIX/lib/pkgconfig" \
    LD_LIBRARY_PATH="$MUSLPREFIX" \
    OPENSSL_STATIC=true \
    OPENSSL_DIR="$MUSLPREFIX" \
    SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt \
    SSL_CERT_DIR=/etc/ssl/certs 
    
# Set up a prefix for musl build libraries, make the linker's job of finding them easier
# Lastly, link some linux-headers for openssl 1.1 (not used herein)
RUN mkdir -p "$MUSLPREFIX" && \
    ln -s /usr/include/x86_64-linux-musl "$MUSLPREFIX/include" && \
    ln -s /usr/lib/x86_64-linux-musl "$MUSLPREFIX/lib" && \
    echo /usr/lib/x86_64-linux-musl >> /etc/ld-musl-x86_64.path && \
    ln -s /usr/include/x86_64-linux-gnu/asm /usr/include/x86_64-linux-musl/asm && \
    ln -s /usr/include/asm-generic /usr/include/x86_64-linux-musl/asm-generic && \
    ln -s /usr/include/linux /usr/include/x86_64-linux-musl/linux

# Download and build official debian openssl package but with musl
# Essential compilation vars
# SSL cert directories get overridden by --prefix and --openssldir
# and they do not match the typical host configurations.
# The SSL_CERT_* vars fix this, but only when inside this container
# musl-compiled binary must point SSL at the correct certs (muslrust/issues/5) elsewhere
RUN export CC=musl-gcc && \
    apt-get source openssl && \
    cd openssl* && \
    ./Configure -static no-shared -fPIC "--prefix=$MUSLPREFIX" "--openssldir=$MUSLPREFIX" linux-x86_64 && \
    env "C_INCLUDE_PATH=/usr/include" make depend && \
    make -j$(nproc) && make install_sw && \
    cd .. && rm -rf openssl*

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
RUN cargo fetch
ENV OPENSSL_DIR=/var/local/musl
RUN cargo check
RUN cargo build
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo test --target=x86_64-unknown-linux-musl

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
RUN cargo check --color always --all-features \
    && echo "CLIPPY -------------------------------------------" \
    && cargo clippy --color always --all-features --target=x86_64-unknown-linux-musl -- -Dclippy::all \
    && echo "TEST -------------------------------------------" \
    && cargo test --color always --all-features  --target=x86_64-unknown-linux-musl \
    && echo "RELEASE ------------------------------------------" \
    && cargo build --color always --release --target=x86_64-unknown-linux-musl

####################################
# Samotop server build
####################################
FROM scratch as server
COPY --from=stable /app/target/*/release/samotop-server /bin/samotop
COPY --from=stable /var/local/musl/bin/openssl /bin/openssl
COPY /samotop-server/ssl /var/ssl
ENV PATH=/bin
VOLUME ["/var/ssl"]
ENTRYPOINT ["/bin/samotop"]
CMD ["--help"]
#USER 1001
