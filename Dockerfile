# 1: Build the exe
FROM rust:latest as builder
WORKDIR /usr/ddv

# 1a: Prepare for static linking
RUN apt-get update && \
    apt-get dist-upgrade -y && \
    apt-get install -y libssl-dev && \
    rustup toolchain add stable

# 1c: Build the exe using the actual source code
COPY Cargo.toml .
COPY src ./src
RUN cargo build --release

# 2: Copy the exe and extra files ("static") to an empty Docker image
FROM debian:10-slim
COPY --from=builder /usr/ddv/target/release/dotdotvote .

COPY apidocs ./apidocs
COPY views ./views
COPY assets ./static

RUN apt-get update && \
    apt-get dist-upgrade -y && \
    apt-get install -y libssl-dev

USER 1000
CMD ["./dotdotvote"]
