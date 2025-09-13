FROM rust:bookworm as builder

RUN apt-get update && apt-get upgrade -y

WORKDIR /kohaku

COPY Cargo.toml ./

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

COPY src src
RUN touch -a -m src/main.rs
RUN cargo build --release

# -----------------------
FROM debian:bookworm

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install libpq5 -y && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

COPY --from=builder /kohaku/target/release/kohaku /kohaku
CMD ["/kohaku"]