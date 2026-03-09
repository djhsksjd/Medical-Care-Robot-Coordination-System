FROM rust:1.85-bookworm AS builder
WORKDIR /app

COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/COMP_2432project /usr/local/bin/COMP_2432project

EXPOSE 3000
CMD ["/usr/local/bin/COMP_2432project"]
