FROM rust:1.94 AS fetcher
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs
RUN echo "fn main() {}" > src/bin/bt-engine.rs
RUN echo "fn main() {}" > src/bin/webapp.rs
RUN cargo fetch


FROM rust:1.94 AS builder
WORKDIR /app

COPY --from=fetcher /usr/local/cargo /usr/local/cargo
COPY . .

RUN cargo build --bin bt-engine --bin webapp


FROM rust:1.94
RUN apt-get update && apt-get install -y \
   ca-certificates \
   build-essential \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo /usr/local/cargo
RUN ln -sf /app/target/debug/bt-engine /usr/local/bin/bt-engine \
 && ln -sf /app/target/debug/webapp /usr/local/bin/webapp

WORKDIR /usr/src/app

CMD ["sleep", "infinity"]
