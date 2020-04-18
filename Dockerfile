# Dockerfile for creating a statically-linked Rust application using docker's
# multi-stage build feature. This also leverages the docker build cache to avoid
# re-downloading dependencies if they have not changed.
FROM rust:1.42.0-stretch AS build

# muslc is required in order to build the rust image.
RUN apt-get update && apt-get -y install ca-certificates cmake musl-tools libssl-dev && rm -rf /var/lib/apt/lists/*

COPY . .
RUN rustup target add x86_64-unknown-linux-musl
# Sets the environment variable for the cargo build command that follows.
ENV PKG_CONFIG_ALLOW_CROSS=1
RUN cargo build --target x86_64-unknown-linux-musl --release


FROM jrottenberg/ffmpeg:3.3-alpine AS ffmpeg

FROM alpine:3.8

COPY --from=ffmpeg / / 

# RUN apk add  --no-cache ffmpeg << do when everything gone to shit

RUN apk --no-cache add ca-certificates
COPY --from=build /target/x86_64-unknown-linux-musl/release/streaming .



CMD ["/streaming"]
