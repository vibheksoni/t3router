FROM rust:1.88-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    clang cmake libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

RUN cargo build --release --bin t3chat

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/t3chat /usr/local/bin/t3chat

LABEL org.opencontainers.image.source="https://github.com/vibheksoni/t3router"
LABEL org.opencontainers.image.description="Rust client library for t3.chat — access 50+ AI models from your terminal"
LABEL org.opencontainers.image.license="MIT"

ENTRYPOINT ["t3chat"]
