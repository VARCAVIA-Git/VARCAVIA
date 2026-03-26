# ═══════════════════════════════════════════════════════════
# VARCAVIA Node — Multi-stage Docker build
# ═══════════════════════════════════════════════════════════
# Risultato: immagine < 50MB con solo il binary + runtime minimo

# ── Stage 1: Build ─────────────────────────────────────────
FROM rust:1.78-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build in release mode
RUN cargo build --release --bin varcavia-node && \
    strip target/release/varcavia-node

# ── Stage 2: Runtime ───────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    useradd -r -s /bin/false varcavia && \
    mkdir -p /data && chown varcavia:varcavia /data

COPY --from=builder /build/target/release/varcavia-node /usr/local/bin/
COPY web/public/ /srv/public/

USER varcavia
WORKDIR /data

EXPOSE 8080 8180

ENV RUST_LOG=info

ENTRYPOINT ["varcavia-node"]
CMD ["--port", "8080", "--data-dir", "/data"]
