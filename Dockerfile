FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
RUN apt-get update && apt-get install -y libopus-dev pkg-config build-essential
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin david-discord-bot-rs
RUN chmod +x /app/target/release/david-discord-bot-rs

# -------- Runtime stage --------
FROM debian:12-slim

ARG DEBIAN_FRONTEND=noninteractive
ENV TZ=America/Chicago
ENV SFX_FILE_PATH=/app/assets/breathing.opus

WORKDIR /app

COPY ./assets/breathing.opus /app/assets/breathing.opus
COPY --from=builder /app/target/release/david-discord-bot-rs /app/

RUN apt-get update && \
    apt-get install -y --no-install-recommends tzdata libopus0 ca-certificates && \
    ln -sf /usr/share/zoneinfo/${TZ} /etc/localtime && \
    echo "${TZ}" > /etc/timezone && \
    dpkg-reconfigure --frontend noninteractive tzdata && \
    apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

CMD ["./david-discord-bot-rs"]