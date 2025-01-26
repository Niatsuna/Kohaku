FROM rust:bookworm as builder

# Prepare Builder
WORKDIR /kohaku

# > Copy necessarry files (Dependencies only)
COPY Cargo.toml ./

# > Create empty main.rs for pure dependency building
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Building : Dependencies (Cache)
RUN cargo build --release

# Building : Application (Cache)
COPY src src
RUN touch -a -m src/main.rs
RUN cargo build --release

# -----------------------
# Executing : Application
FROM debian:bookworm
RUN apt-get update && apt-get install libpq5 -y

COPY --from=builder /kohaku/target/release/kohaku /kohaku
CMD ["/kohaku"]