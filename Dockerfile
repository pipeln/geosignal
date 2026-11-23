# syntax=docker/dockerfile:1.7

FROM rust:1-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        clang \
        cmake \
        libclang-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY public ./public

RUN cargo build --release --locked --no-default-features --bin geosignal

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libgomp1 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 user

WORKDIR /home/user/app

COPY --from=builder /app/target/release/geosignal /usr/local/bin/geosignal
COPY --from=builder --chown=user:user /app/public ./public

COPY --chown=user:user LICENSE NOTICE MODEL_LICENSES.md THIRD_PARTY_LICENSES.md ./
COPY --chown=user:user licenses ./licenses
COPY --chown=user:user docker/entrypoint.sh /usr/local/bin/geosignal-entrypoint

RUN chmod 0755 /usr/local/bin/geosignal /usr/local/bin/geosignal-entrypoint

ENV HOME=/home/user \
    GEOSIGNAL_ASSETS=/home/user/app/public \
    GEOSIGNAL_BIND=0.0.0.0 \
    GEOSIGNAL_PUBLIC=1 \
    PORT=7860

USER user
EXPOSE 7860

ENTRYPOINT ["/usr/local/bin/geosignal-entrypoint"]
CMD ["geosignal"]
