FROM rust:1.94 AS fetcher
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs
RUN echo "fn main() {}" > src/bin/bt-engine.rs
RUN cargo fetch


FROM rust:1.94 AS builder
WORKDIR /app

COPY --from=fetcher /usr/local/cargo /usr/local/cargo
COPY . .

RUN cargo build --release && \
 cargo install cargo-watch --root /usr/local


FROM rust:1.94
RUN apt-get update && apt-get install -y \
   ca-certificates \
   build-essential \
 && rm -rf /var/lib/apt/lists/*


COPY --from=builder /usr/local/cargo /usr/local/cargo
COPY --from=builder /usr/local/bin/cargo-watch /usr/local/bin/cargo-watch
COPY --from=builder /app/target/release /usr/local/bin/

WORKDIR /usr/src/app

CMD cargo watch -i logs/ -x "build --bin bt-engine" -s "cp target/debug/bt-engine /usr/local/bin/bt-engine"
