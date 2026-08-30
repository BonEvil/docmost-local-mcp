# Dockerfile — used by Glama's (glama.ai) build + introspection sandbox to run the
# MCP server in a headless container. This is NOT the primary distribution path:
# Atlas production uses the verified fork installer and an absolute binary path.
# This image exists so automated MCP directories can build the
# server, start it, and perform the tools/list introspection exchange.
#
# The server is built with `--no-default-features`, which disables the
# `native-webview` feature (tao/wry, GTK/WebKit). Auth therefore always uses the
# browser-fallback path, and the container needs no GUI libraries. Introspection
# (initialize + tools/list) requires neither a Docmost instance nor authentication.

# ---- build stage ----
FROM rust:1.98.0-slim-bookworm@sha256:1469a27c125cb5a3aebfa4f4e4665d935b02fb72cc093b2c974b3d740e43f157 AS builder
WORKDIR /build
# reqwest uses rustls (no OpenSSL). keyring links against dbus on Linux.
COPY config/debian-snapshot.sources /etc/apt/sources.list.d/debian.sources
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config=1.8.1-1 \
        libdbus-1-dev=1.14.10-1~deb12u1 \
    && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --locked --release --no-default-features

# ---- runtime stage ----
FROM debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171 AS runtime
COPY config/debian-snapshot.sources /etc/apt/sources.list.d/debian.sources
RUN apt-get update && apt-get install -y --no-install-recommends \
        libdbus-1-3=1.14.10-1~deb12u1 \
        ca-certificates=20250419~deb12u1 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/docmost-local-mcp /usr/local/bin/docmost-local-mcp
# The MCP server speaks JSON-RPC over stdio.
ENTRYPOINT ["docmost-local-mcp"]
