FROM lukemathwalker/cargo-chef:0.1.71-rust-1.85.1-alpine3.21 AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM rust:1.85.1-alpine3.21 AS build

RUN rustup target add x86_64-unknown-linux-musl
RUN apk add --no-cache build-base pkgconfig dbus-dev "libressl-dev<4.0.0" protoc protobuf-dev

WORKDIR /app

RUN cargo install cargo-chef

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

COPY . .
RUN cargo build --target x86_64-unknown-linux-musl --release

RUN strip /app/target/x86_64-unknown-linux-musl/release/pdf-rendering-srv


FROM alpine:3.21

RUN apk --update --upgrade --no-cache add fontconfig font-noto font-noto-emoji font-liberation \
    && fc-cache -f \
    && fc-list | sort

# Don't install M$ fonts because of license issues
# RUN apk --update add fontconfig msttcorefonts-installer \
#     && update-ms-fonts \
#     && fc-cache -f

RUN apk add --no-cache chromium

WORKDIR /app

COPY --from=build /app/target/x86_64-unknown-linux-musl/release/pdf-rendering-srv /app/pdf-rendering-srv
COPY ./cfg /app/cfg

ENV NODE_ENV=production

EXPOSE 50051

CMD ["/app/pdf-rendering-srv"]
