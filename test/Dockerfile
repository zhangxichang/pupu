FROM rust:latest AS build
WORKDIR /src
COPY Cargo.lock /src/Cargo.lock
COPY runtime /src/runtime
COPY crates /src/crates
COPY test /src/test
RUN mv /src/test/Cargo.workspace.toml /src/Cargo.toml
RUN cargo build --release -p test-peer

FROM debian:latest AS peer
RUN apt update && apt install -y ca-certificates iproute2
COPY --from=build /src/target/release/test-peer /usr/local/bin/test-peer
ENTRYPOINT ["test-peer"]
