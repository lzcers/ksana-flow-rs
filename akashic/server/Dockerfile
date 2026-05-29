# syntax=docker/dockerfile:1.7
FROM rust:1.96.0 AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY Cargo.toml Cargo.lock ./
COPY agent ./agent
COPY akashic/ecs ./akashic/ecs
COPY akashic/server ./akashic/server
COPY ksana/flow ./ksana/flow
COPY ksana/nodes ./ksana/nodes
COPY ksana/server ./ksana/server

RUN --mount=type=secret,id=cargo_config,target=/run/secrets/cargo_config,required=false \
    if [ -f /run/secrets/cargo_config ]; then \
    mkdir -p /usr/local/cargo && \
    cp /run/secrets/cargo_config /usr/local/cargo/config.toml; \
    fi && \
    cargo build --release -p akashic-server && \
    rm -f /usr/local/cargo/config.toml

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /workspace/target/release/akashic-server /usr/local/bin/akashic-server

EXPOSE 3001

CMD ["akashic-server"]
