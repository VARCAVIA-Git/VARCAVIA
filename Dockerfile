FROM rust:latest AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin varcavia-node

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/varcavia-node /usr/local/bin/
RUN useradd -r -s /bin/false varcavia && mkdir -p /app/data && chown varcavia:varcavia /app/data
USER varcavia
EXPOSE 8080
CMD ["varcavia-node", "--data-dir", "/app/data"]
