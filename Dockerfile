FROM rust:1.83.0 as builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch

COPY . .

RUN cargo build --release

RUN strip target/release/rust-sky-answ

FROM ubuntu:22.04
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/rust-sky-answ .

CMD ["./rust-sky-answ"]