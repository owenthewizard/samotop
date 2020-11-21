FROM rust:latest as dependencies
#RUN cargo install toml

# install wildq
RUN VERSION=$(curl -s "https://api.github.com/repos/ahmet2mir/wildq/releases/latest" | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/') \
    && curl -sL https://github.com/ahmet2mir/wildq/releases/download/v${VERSION}/wildq_${VERSION}-1_amd64.deb -o wildq_${VERSION}-1_amd64.deb \
    && sudo dpkg -i wildq_${VERSION}-1_amd64.deb

WORKDIR /app
RUN USER=root cargo new --bin dependencies
WORKDIR /app/dependencies
COPY samotop-server/Cargo.toml .

RUN find . -name Cargo.toml | \
    xargs -I{} toml get {} dependencies | \
    jq -s 'add | to_entries | .[] | select(.value|type=="string" or (.value.path?|not))' | \
    wildq -i json -o toml
