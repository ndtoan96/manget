FROM golang as lib_builder
WORKDIR /app
ADD libkepubify/go.mod go.mod
ADD libkepubify/go.sum go.sum
ADD libkepubify/main.go main.go
RUN go build -o kepubify.lib -buildmode=c-shared .

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
WORKDIR /app
RUN apt update && apt install -y openssl curl
COPY --from=builder /usr/local/cargo/bin/manget_server /usr/local/bin/manget_server
COPY --from=lib_builder /app/kepubify.lib libkepubify/kepubify.lib
EXPOSE 8080
CMD ["manget_server"]
