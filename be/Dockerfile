FROM rust:1.93-slim-bookworm AS builder

WORKDIR /app/be

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY be/Cargo.toml be/Cargo.lock ./
COPY be/src ./src

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

WORKDIR /app/be

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/be/target/release/be /usr/local/bin/rapidbro-be
COPY be/bus_data /app/be/bus_data

ENV REDIS_URL=redis://redis:6379/
ENV BUS_TTL_SECONDS=300
ENV STALE_AFTER_SECONDS=20
ENV STATIONARY_WINDOW_SECONDS=300

EXPOSE 3030

CMD ["/usr/local/bin/rapidbro-be"]
