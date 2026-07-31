# Dockerfile — Graphify Pro
#
# Build:
#   docker build -t graphify-pro .
#
# Run:
#   docker run --rm -v $(pwd):/workspace graphify-pro build /workspace
#
# Serve:
#   docker run --rm -p 8080:8080 -v $(pwd):/workspace graphify-pro serve /workspace/graphify-out/graph.json

FROM rust:1.82-slim-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN cargo build --release && \
    cp target/release/graphify /usr/local/bin/graphify

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates git \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/graphify /usr/local/bin/graphify

WORKDIR /workspace
ENTRYPOINT ["graphify"]
CMD ["--help"]
