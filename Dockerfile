# ── Build stage ───────────────────────────────────────────────────────────────
FROM rust:1.94-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY templates ./templates

RUN cargo build --release

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# yt-dlp binary (updated separately from OS packages)
RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp \
    -o /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp

COPY --from=builder /build/target/release/broll-rs /usr/local/bin/broll-rs

WORKDIR /app

VOLUME ["/app/downloads"]

EXPOSE 8000

ENV DATABASE_URL=sqlite:///app/library.db
ENV DOWNLOADS_DIR=/app/downloads

CMD ["broll-rs"]
