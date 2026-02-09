FROM rust:1.90

RUN cargo install sqlx-cli --no-default-features --features postgres

WORKDIR /usr/src/presley
COPY . .

RUN cargo install --profile release --path . 
CMD ["presley-bot"]
