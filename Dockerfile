FROM rust:1.68.0 as builder

WORKDIR /app

COPY . .

RUN cargo build --release

FROM ubuntu:22.04

COPY --from=builder /app/target/release/health-checks /


RUN apt update && apt-get install ca-certificates -y && update-ca-certificates

RUN apt install iputils-ping -y && apt install wget -y

RUN wget http://nz2.archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1f-1ubuntu2.17_amd64.deb

RUN dpkg -i libssl1.1_1.1.1f-1ubuntu2.17_amd64.deb

RUN mkdir -p /config

CMD ["/health-checks", "-c", "/config/config.json"]