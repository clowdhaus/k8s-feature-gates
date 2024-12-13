# syntax=docker/dockerfile:1

ARG TARGETARCH=amd64

ARG RUST_VERSION=1.82
ARG OS_VERSION=bookworm

FROM --platform=linux/${TARGETARCH} rust:${RUST_VERSION}-${OS_VERSION} AS base
RUN cargo install sccache cargo-chef@0.1.68 --locked
ENV RUSTC_WRAPPER=sccache SCCACHE_DIR=/sccache

# ----------------------------------------------------------------------------

FROM base AS planner
WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
  cargo chef prepare --recipe-path recipe.json

# ----------------------------------------------------------------------------

FROM base AS builder
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
  cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
  cargo build --release

# ----------------------------------------------------------------------------

FROM --platform=linux/${TARGETARCH} cgr.dev/chainguard/glibc-dynamic:latest AS runtime

COPY --from=builder /usr/lib/*/libssl.so.3 /usr/lib/libssl.so.3
COPY --from=builder /usr/lib/*/libcrypto.so.3 /usr/lib/libcrypto.so.3
COPY --from=builder --chown=nonroot:nonroot /app/target/release/k8sfg /k8sfg

CMD ["/k8sfg"]
