FROM rust:1.23.0

WORKDIR /usr/src/hostname
COPY . .

EXPOSE 9000
RUN cargo install

CMD ["hostname-service"]

