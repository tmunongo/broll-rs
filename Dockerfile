# ── Build stage ───────────────────────────────────────────────────────────────
FROM rust:1.94-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev gcc libc6-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache dependencies by creating a dummy main.rs
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy actual source code
COPY src ./src
COPY templates ./templates

# Touch main.rs to ensure Cargo realizes it needs to be rebuilt
RUN touch src/main.rs
RUN cargo build --release

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

# Install Python 3 (required for the arch-independent yt-dlp zipapp), ffmpeg, and curl
RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg curl ca-certificates python3 \
    && rm -rf /var/lib/apt/lists/*

# Platform-independent yt-dlp (runs natively on both AMD64 and ARM64 via python3)
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
