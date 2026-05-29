# syntax=docker/dockerfile:1.7
FROM rust:1.78-slim AS build
WORKDIR /src
RUN apt-get update && apt-get install -y pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml ./
COPY crates ./crates
RUN cargo build --release --bin sentry-gateway

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/sentry-gateway /usr/local/bin/sentry-gateway
COPY config.example.yaml /etc/agentsentry/config.yaml
ENV SENTRY_GW_CONFIG=/etc/agentsentry/config.yaml
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/sentry-gateway"]
