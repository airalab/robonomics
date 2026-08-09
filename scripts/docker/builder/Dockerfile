FROM rust:1.93.1-alpine

RUN apk update && \
    apk add --no-cache build-base clang-dev clang-static llvm-dev llvm-static protoc

RUN ln -s /usr/bin/llvm-config-21 /usr/bin/llvm-config

WORKDIR /build
