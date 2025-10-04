FROM alpine:3.14 as builder

RUN apk add --no-cache curl opus-dev

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app
COPY . .

RUN cargo build --release

# Runtime image
FROM alpine:3.14

WORKDIR /app
COPY --from=builder /app/target/release/david-discord-bot-rs /app/
RUN chmod +x david-discord-bot-rs

CMD ["./david-discord-bot-rs"]