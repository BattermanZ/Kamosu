# Kamosu ships as one executable in one distroless image (ADR 0028): no shell,
# no Node, no browser, no model. The binary checks its own health because there
# is no curl to do it. One mount (/data) and one port (5266) are the whole install.
#
# **No model** is the literal truth and not a simplification (ADR 0029): the
# ONNX Runtime that Meaning Search would use is linked into the binary, and the
# weights are not here at all. An Operator who wants them accepts Google's terms
# through an ordinary Operation and Kamosu downloads them into /data — so this
# image carries nobody else's licence, and installs with no network at all.

# The interface is built here and compiled *into* the binary below, never copied
# beside it. Node exists in this stage and nowhere else: the artefact is the
# version, so a half-upgraded install — new binary, last month's assets — is not
# something an operator can arrive at.
FROM node:24-slim AS ui
WORKDIR /ui
COPY ui/package.json ui/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY ui ./
RUN npm run build

FROM rust:1.95-slim AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock rust-toolchain.toml build.rs ./
COPY src ./src
# Embedded at compile time by src/design_tokens.rs (include_str!/include_bytes!):
# the generated stylesheet, the self-hosted fonts and the icon PNGs. Their
# source in ui/ is not needed — the output is committed and verified fresh by
# `just check`.
COPY assets ./assets
# Embedded at compile time by src/interface.rs: the built Svelte app. build.rs
# refuses a release build without it, so this COPY is load-bearing, not optional.
COPY --from=ui /ui/build ./ui/build
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=build /build/target/release/kamosu /app/kamosu
EXPOSE 5266
# Exec form: distroless has no shell to interpret a command string.
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s CMD ["/app/kamosu", "health-check"]
ENTRYPOINT ["/app/kamosu"]
CMD ["serve"]
