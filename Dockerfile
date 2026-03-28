FROM rust:latest AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin varcavia-node

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/varcavia-node /usr/local/bin/
RUN mkdir -p /data/varcavia /app/data && chmod 777 /data /data/varcavia
EXPOSE 8080
CMD ["sh", "-c", "mkdir -p /data/varcavia 2>/dev/null; exec varcavia-node --data-dir /app/data"]
