FROM rust:1.90

WORKDIR /usr/src/presley
COPY . .

RUN cargo install sqlx-cli --no-default-features --features postgres
RUN cargo install --path . 
CMD ["presley"]
