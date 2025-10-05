# -------- Build stage --------
#FROM alpine:3.14 as builder

# Install dependencies
#RUN apk add --no-cache curl opus-dev build-base git

# Install Rust
#RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
#ENV PATH="/root/.cargo/bin:${PATH}"

# build deps
#WORKDIR /app

#COPY Cargo.toml Cargo.lock ./

#RUN mkdir src && echo "fn main() {}" > src/main.rs
#RUN cargo build --release
#RUN rm -rf src

# build app
#COPY . .

#RUN cargo build --release


FROM lukemathwalker/cargo-chef:latest-rust-1-alpine AS chef
RUN apk add --no-cache opus-dev
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
RUN cargo build --release --bin app

# -------- Runtime stage --------
FROM scratch

WORKDIR /app
COPY --from=builder /app/target/release/david-discord-bot-rs /app/
RUN chmod +x david-discord-bot-rs

ENV TZ=America/Chicago

CMD ["./david-discord-bot-rs"]