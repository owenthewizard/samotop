FROM rust:latest as dependencies

# install toml
RUN cargo install toml-cli

# install wildq
RUN VERSION=$(curl -s "https://api.github.com/repos/ahmet2mir/wildq/releases/latest" | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/') \
    && curl -sL https://github.com/ahmet2mir/wildq/releases/download/v${VERSION}/wildq_${VERSION}-1_amd64.deb -o wildq_${VERSION}-1_amd64.deb \
    && dpkg -i wildq_${VERSION}-1_amd64.deb

RUN apt-get update && apt-get install -y \
   jq

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
RUN USER=root cargo new --bin dependencies && \
    mv dependencies/Cargo.toml dependencies/Cargo.template.toml && \
    cat dependencies/Cargo.template.toml | wildq -M -i toml -o toml 'del(.dependencies)' > dependencies/Cargo.toml
RUN find . -name Cargo.toml -mindepth 1 | \
    xargs -I{} toml get {} dependencies | \
    jq -s 'add | to_entries | .[] |  select((.value|type=="string") or (.value.path?|not))' | \
    jq -s 'from_entries' | \
    #jq -s 'add | to_entries | .[] | select(.value|type=="string" or (.value.path?|not)) ' 
    wildq -M -i json -o toml '{"dependencies": .}' | tee -a dependencies/Cargo.toml && \
    find . -name Cargo.toml -mindepth 1 | \
    xargs -I{} toml get {} dev-dependencies | \
    jq -s 'add | to_entries | .[] |  select((.value|type=="string") or (.value.path?|not))' | \
    jq -s 'from_entries' | \
    #jq -s 'add | to_entries | .[] | select(.value|type=="string" or (.value.path?|not)) ' 
    wildq -M -i json -o toml '{"dev-dependencies": .}' | tee -a dependencies/Cargo.toml && \
    cd dependencies && cargo fetch && cat Cargo.lock
