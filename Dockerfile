FROM mcr.microsoft.com/devcontainers/rust:latest
RUN apt-get update && \
  apt-get install -y libssl-dev protobuf-compiler libclang-dev clang cmake build-essential && \
  rustup target add wasm32-unknown-unknown && \
  git config --global safe.directory '*'