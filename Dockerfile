FROM rust as builder
WORKDIR /app
ADD manget_server/src ./manget_server/src
ADD manget_server/Cargo.toml ./manget_server/Cargo.toml
ADD /manget/src ./manget/src
ADD /manget/Cargo.toml ./manget/Cargo.toml
WORKDIR /app/manget_server
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN cargo install --path .

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/manget_server /usr/local/bin/manget_server
CMD ["manget_server"]