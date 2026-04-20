FROM rust:1.87-bookworm AS builder

WORKDIR /build

# Cache dependencies
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release 2>/dev/null || true \
    && rm -rf src

# Build the real binary
COPY src/ src/
RUN cargo build --release

# Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --uid 1000 --create-home app

COPY --from=builder /build/target/release/starcitizen-api /usr/local/bin/starcitizen-api

USER app
EXPOSE 8080

ENV DATA_DIR=/data
ENV BIND_ADDR=0.0.0.0:8080
ENV RUST_LOG=starcitizen_api=info,tower_http=info

CMD ["starcitizen-api"]
