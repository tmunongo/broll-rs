# ── Build stage ───────────────────────────────────────────────────────────────
FROM golang:1.27-bookworm AS builder

WORKDIR /build

COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN CGO_ENABLED=1 GOOS=linux go build -ldflags="-s -w" -o broll-rs main.go

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

COPY --from=builder /build/broll-rs /usr/local/bin/broll-rs

WORKDIR /app

VOLUME ["/app/downloads"]

EXPOSE 8000

ENV DATABASE_URL=sqlite:///app/library.db
ENV DOWNLOADS_DIR=/app/downloads

CMD ["broll-rs"]
