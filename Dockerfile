FROM rust:1.98.1 AS fetcher
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && echo "" > src/lib.rs
RUN echo "fn main() {}" > src/bin/bt-engine.rs
RUN cargo fetch


FROM rust:1.98.1 AS builder
WORKDIR /app

COPY --from=fetcher /usr/local/cargo /usr/local/cargo
COPY . .

RUN cargo build --bin bte


FROM rust:1.98.1
RUN apt-get update && apt-get install -y \
   ca-certificates \
   build-essential \
   mold \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo /usr/local/cargo
RUN ln -sf /app/target/debug/bte /usr/local/bin/bte

WORKDIR /usr/src/app

CMD ["sleep", "infinity"]
