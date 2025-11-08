FROM rust:1.90

WORKDIR /usr/src/presley
COPY . .

RUN cargo install sqlx-cli
RUN cargo sqlx prepare
RUN cargo install --path . 
CMD ["presley"]
