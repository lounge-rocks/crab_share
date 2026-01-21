FROM rust:1.92.0-slim-trixie AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev zip
COPY . /crab_share
WORKDIR /crab_share
RUN cargo install --path .

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates zip && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/crab_share /bin/crab_share
