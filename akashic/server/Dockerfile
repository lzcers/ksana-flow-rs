FROM rust:1.96 AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY Cargo.toml Cargo.lock ./
COPY agent ./agent
COPY akashic/ecs ./akashic/ecs
COPY akashic/server ./akashic/server

RUN cargo build --locked --release -p akashic-server

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /workspace/target/release/akashic-server /usr/local/bin/akashic-server

EXPOSE 3001

CMD ["akashic-server"]
