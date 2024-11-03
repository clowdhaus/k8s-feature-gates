FROM --platform=linux/amd64 rust:bookworm AS base
RUN cargo install sccache cargo-chef
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

FROM --platform=linux/amd64 debian:bookworm-slim
COPY --from=builder --chown=nonroot:nonroot /app/target/release/k8sfg /k8sfg

RUN apt-get update && apt-get install -y \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*
CMD ["./k8sfg"]
