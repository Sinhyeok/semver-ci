# syntax=docker/dockerfile:1

ARG VARIANT=alpine

# Track published patch updates within Rust 1.98 for the bootstrap image.
# rust-toolchain.toml selects Rust and Cargo 1.98.1 for the application build.
FROM rust:1.98-bookworm AS build-base

RUN apt-get update && \
    apt-get install -y --no-install-recommends cmake musl-tools && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY rust-toolchain.toml ./
RUN rustup show active-toolchain

FROM build-base AS build-alpine
ENV CARGO_BUILD_TARGET=x86_64-unknown-linux-musl
# Debian's musl-tools supplies musl-gcc but uses the system binutils.
ENV CC_x86_64_unknown_linux_musl=musl-gcc \
    AR_x86_64_unknown_linux_musl=ar \
    RANLIB_x86_64_unknown_linux_musl=ranlib

FROM build-base AS build-debian
ENV CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu

FROM build-${VARIANT} AS build
ARG CARGO_PROFILE=release

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build --locked --profile "$CARGO_PROFILE" && \
    if [ "$CARGO_PROFILE" = dev ]; then CARGO_PROFILE=debug; fi && \
    cp "target/$CARGO_BUILD_TARGET/$CARGO_PROFILE/svci" /usr/local/bin/svci

FROM alpine:3.24 AS runtime-alpine
RUN apk add --no-cache ca-certificates curl

FROM debian:bookworm-slim AS runtime-debian
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl libgcc-s1 && \
    rm -rf /var/lib/apt/lists/*

FROM runtime-${VARIANT} AS action
COPY --from=build /usr/local/bin/svci /usr/local/bin/svci
WORKDIR /app
ENTRYPOINT ["svci"]
