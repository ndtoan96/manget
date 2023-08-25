FROM rust as builder
WORKDIR /app
ADD manget_server/src ./manget_server/src
ADD manget_server/Cargo.toml ./manget_server/Cargo.toml
ADD /manget/src ./manget/src
ADD /manget/Cargo.toml ./manget/Cargo.toml
WORKDIR /app/manget_server
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN cargo install --path .

FROM ubuntu:latest
RUN apt update && apt install -y openssl curl
COPY --from=builder /usr/local/cargo/bin/manget_server /usr/local/bin/manget_server
EXPOSE 8080
CMD ["manget_server"]
