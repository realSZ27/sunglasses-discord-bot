FROM lukemathwalker/cargo-chef:latest-rust-1-alpine AS chef
RUN apk add --no-cache opus-dev build-base musl-dev
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
FROM alpine:3.14

WORKDIR /app
RUN apk add --no-cache tzdata
COPY ./assets/breathing.opus /app/assets/breathing.opus
COPY --from=builder /app/target/release/david-discord-bot-rs /app/

ENV TZ=America/Chicago
ENV SFX_FILE_PATH=/app/assets/breathing.opus

CMD ["./david-discord-bot-rs"]