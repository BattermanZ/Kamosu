# syntax=docker.io/docker/dockerfile:1.7

# Kamosu ships as one executable in one distroless image (ADR 0028): no shell,
# no Node, no browser, no model. The binary checks its own health because there
# is no curl to do it. One mount (/data) and one port (5266) are the whole install.
#
# **No model** is the literal truth and not a simplification (ADR 0029): the
# ONNX Runtime that Meaning Search would use is linked into the binary, and the
# weights are not here at all. An Operator who wants them accepts Google's terms
# through an ordinary Operation and Kamosu downloads them into /data — so this
# image carries nobody else's licence, and installs with no network at all.
#
# Every image name is written in full (docker.io/library/…): Podman refuses or
# prompts on a short name like `rust:1.95-slim`, where Docker silently assumes
# Docker Hub. Build with Podman as `podman build --format docker`, or the
# HEALTHCHECK below is dropped. The OCI image format has no field for it.

# The interface is built here and compiled *into* the binary below, never copied
# beside it. Node exists in this stage and nowhere else: the artefact is the
# version, so a half-upgraded install — new binary, last month's assets — is not
# something an operator can arrive at.
FROM docker.io/library/node:24-slim AS ui
WORKDIR /ui
COPY ui/package.json ui/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY ui ./
RUN npm run build

# The Rust version is rust-toolchain.toml's; this tag only saves rustup a
# download. Its Debian release must match the runtime's below (both 13): a
# binary linked against a newer glibc than the one it runs on does not start.
FROM docker.io/library/rust:1.95-slim AS chef
# ONNX Runtime is C++, so linking it needs libstdc++, which the slim image
# leaves out. The runtime image below already carries the shared library.
RUN apt-get update && apt-get install -y --no-install-recommends g++ \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --version 0.1.78 --locked
WORKDIR /build

# cargo-chef splits the build in two. `prepare` reduces the project to the
# list of libraries it depends on; `cook` compiles only those. The list
# changes when Cargo.lock does, which is rarely, so the compiled libraries
# stay a cached image layer across every change to Kamosu's own code.
FROM chef AS planner
COPY Cargo.toml Cargo.lock rust-toolchain.toml build.rs ./
COPY src ./src
# Never compiled here, but Cargo.toml names the corpus tests as [[test]]
# targets, and Cargo refuses to build anything while a named target's file
# is missing.
COPY tests ./tests
RUN cargo chef prepare --recipe-path recipe.json

# Only the downloads sit in cache mounts. The compiled libraries are written
# into target/ inside this layer on purpose: a cache mount is not part of the
# image, so a build that put target/ in one would keep nothing here, and a
# machine without the cache (a fresh builder, a cleared store) would recompile
# every library despite this stage being cached.
FROM chef AS dependencies
COPY rust-toolchain.toml ./
COPY --from=planner /build/recipe.json recipe.json
RUN --mount=type=cache,id=kamosu-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=kamosu-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    cargo chef cook --release --locked --recipe-path recipe.json

FROM dependencies AS build
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
COPY tests ./tests
# Embedded at compile time by src/design_tokens.rs (include_str!/include_bytes!):
# the generated stylesheet, the self-hosted fonts and the icon PNGs. Their
# source in ui/ is not needed — the output is committed and verified fresh by
# `just check`.
COPY assets ./assets
# Embedded at compile time by src/interface.rs: the built Svelte app. build.rs
# refuses a release build without it, so this COPY is load-bearing, not optional.
COPY --from=ui /ui/build ./ui/build
# `--locked` builds exactly Cargo.lock.
RUN --mount=type=cache,id=kamosu-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=kamosu-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    cargo build --release --locked \
    && cp target/release/kamosu /kamosu \
    && mkdir /data-empty

# Pinned by digest so a rebuild months from now runs on the same base; move the
# digest deliberately, the way a dependency is upgraded. `nonroot` runs Kamosu
# as uid 65532 rather than root, so an escape from the process lands in an
# account that owns nothing but /data.
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:54df941ed0d06a1bd95ef5e0ce391fd8d9f94b64782dc9a60062727849ee3f97
COPY --from=build /kamosu /app/kamosu
# /data exists in the image, owned by the user who writes it, so a fresh named
# volume mounted there starts out writable. A bind-mounted host folder brings
# its own owner instead: under rootless Podman, give it to this user with
# `podman unshare chown 65532:65532 <folder>`, or run with
# `--userns=keep-id:uid=65532,gid=65532` so it stays yours on the host.
COPY --from=build --chown=65532:65532 /data-empty /data
VOLUME /data
EXPOSE 5266
USER 65532:65532
# Exec form: distroless has no shell to interpret a command string.
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s CMD ["/app/kamosu", "health-check"]
ENTRYPOINT ["/app/kamosu"]
CMD ["serve"]
