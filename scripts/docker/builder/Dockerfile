FROM rust:1.93.1-alpine

RUN apk update && \
    apk add --no-cache build-base clang-dev clang-static llvm-dev llvm-static \
        ncurses-static protoc

WORKDIR /build
