FROM rust:1.82-slim-bookworm AS builder
WORKDIR /build

# Cache dependency compilation separately from source changes.
COPY Cargo.toml Cargo.lock ./
COPY src-tauri/Cargo.toml src-tauri/Cargo.toml

# Create stub src files so cargo can resolve deps without full source.
RUN mkdir -p src-tauri/src/bin && \
    echo 'fn main(){}' > src-tauri/src/main.rs && \
    echo 'fn main(){}' > src-tauri/src/bin/anchor.rs && \
    echo '' > src-tauri/src/lib.rs && \
    cargo build --release --bin anchor 2>/dev/null || true

# Now copy the real source and do the proper build.
COPY src-tauri/src src-tauri/src
COPY src-tauri/build.rs src-tauri/build.rs
RUN touch src-tauri/src/main.rs src-tauri/src/bin/anchor.rs && \
    cargo build --release --bin anchor

# ── Runtime image ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/anchor /usr/local/bin/anchor

# Persistent volumes — blob storage and TLS certificates.
VOLUME ["/var/lib/p2pstud", "/etc/letsencrypt"]

# Relay (QUIC + TCP on 443) and certbot renewal (HTTP on 80).
EXPOSE 443/udp 443/tcp 80/tcp

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD ["/usr/local/bin/anchor", "--health-check"]

ENTRYPOINT ["/usr/local/bin/anchor"]
CMD ["--data-dir", "/var/lib/p2pstud", "--no-relay"]

