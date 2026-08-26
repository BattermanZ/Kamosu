# Kamosu ships as one executable in one distroless image (ADR 0028): no shell,
# no Node, no browser, no model. The binary checks its own health because there
# is no curl to do it. One mount (/data) and one port (5266) are the whole install.

FROM rust:1.95-slim AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src
# Embedded at compile time by src/design_tokens.rs (include_str!/include_bytes!):
# the generated stylesheet, the self-hosted fonts and the icon PNGs. ui/ itself
# is not needed — its output is committed and verified fresh by `just check`.
COPY assets ./assets
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=build /build/target/release/kamosu /app/kamosu
EXPOSE 5266
# Exec form: distroless has no shell to interpret a command string.
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s CMD ["/app/kamosu", "health-check"]
ENTRYPOINT ["/app/kamosu"]
CMD ["serve"]
