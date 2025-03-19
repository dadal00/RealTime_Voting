FROM rust:1.78-bookworm AS builder

RUN apt-get update && \
    apt-get install -y \
    cmake \
    build-essential \
    pkg-config \
    libssl-dev \
    zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release

ARG RUST_PORT

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y \
    ca-certificates \
    libssl3 \
    zlib1g \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/counter /app/
EXPOSE $RUST_PORT
CMD ["/app/counter"]
