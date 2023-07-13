# syntax=docker/dockerfile:1

# Comments are provided throughout this file to help you get started.
# If you need more help, visit the Dockerfile reference guide at
# https://docs.docker.com/engine/reference/builder/

################################################################################
# Create a stage for building the application.

FROM rust:slim-bookworm AS build
WORKDIR /app

RUN set -eux && apt update && apt install -y pkg-config libssl-dev curl xz-utils clang unzip && rm -rf /var/lib/apt/lists/*
RUN set -eux && mkdir /root/build

ARG TIGERBEETLE_VERSION=0.13.49
ADD https://github.com/tigerbeetledb/tigerbeetle/archive/refs/tags/$TIGERBEETLE_VERSION.zip tigerbeetle.zip
RUN set -eux && unzip tigerbeetle.zip && mv tigerbeetle-$TIGERBEETLE_VERSION tigerbeetle
RUN set -eux && cd tigerbeetle && ./scripts/install.sh && mv tigerbeetle /root/build && cd .. && rm -r tigerbeetle

# Build the application.
# Leverage a cache mount to /usr/local/cargo/registry/
# for downloaded dependencies and a cache mount to /app/target/ for 
# compiled dependencies which will speed up subsequent builds.
# Leverage a bind mount to the src directory to avoid having to copy the
# source code into the container. Once built, copy the executable to an
# output directory before the cache mounted /app/target is unmounted.
RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=sys,target=sys \
    --mount=type=bind,source=core,target=core \
    --mount=type=bind,source=examples,target=examples \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    <<EOF
set -e
mkdir /root/build/debug/examples -p
mkdir /root/build/release/examples -p
cargo build --workspace --examples
cargo build --workspace --examples -r
cp ./target/debug/examples/c_port_low_level /root/build/debug/examples
cp ./target/debug/examples/c_port_high_level /root/build/debug/examples
cp ./target/release/examples/c_port_low_level /root/build/release/examples
cp ./target/release/examples/c_port_high_level /root/build/release/examples
EOF

################################################################################
# Create a new stage for running the application that contains the minimal
# runtime dependencies for the application. This often uses a different base
# image from the build stage where the necessary files are copied from the build
# stage.
#
# The example below uses the debian bullseye image as the foundation for running the app.
# By specifying the "bullseye-slim" tag, it will also use whatever happens to be the
# most recent version of that tag when you build your Dockerfile. If
# reproducability is important, consider using a digest
# (e.g., debian@sha256:ac707220fbd7b67fc19b112cee8170b41a9e97f703f588b2cdbbcdcecdd8af57).
FROM debian:bookworm-slim AS final

RUN set -eux && apt update && apt install -y valgrind && rm -rf /var/lib/apt/lists/*

# # Create a non-privileged user that the app will run under.
# # See https://docs.docker.com/develop/develop-images/dockerfile_best-practices/#user
# ARG UID=10001
# RUN adduser \
#     --disabled-password \
#     --gecos "" \
#     --home "/nonexistent" \
#     --shell "/sbin/nologin" \
#     --no-create-home \
#     --uid "${UID}" \
#     appuser
# USER appuser

COPY --from=build /root/build /root/
RUN set -eux && /root/tigerbeetle format --cluster=0 --replica=0 --replica-count=1 /root/db
CMD ["/root/tigerbeetle", "start", "--addresses=3000", "/root/db"]
