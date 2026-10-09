FROM rust:1.82-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash appuser
USER appuser

WORKDIR /app
COPY --from=builder /app/target/release/ringseal /app/ringseal

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD curl -f http://localhost:8080/api/v1/health || exit 1

CMD ["/app/ringseal"]
