# -------- Build stage --------
FROM alpine:3.14 as builder

# Install dependencies
RUN apk add --no-cache curl opus-dev build-base git

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# build deps
WORKDIR /app

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# build app
COPY . .

RUN cargo build --release

# -------- Runtime stage --------
FROM scratch

WORKDIR /app
COPY --from=builder /app/target/release/david-discord-bot-rs /app/
RUN chmod +x david-discord-bot-rs

ENV TZ=America/Chicago

CMD ["./david-discord-bot-rs"]