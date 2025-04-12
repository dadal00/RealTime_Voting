FROM rust:1-alpine3.21

ENV RUSTFLAGS="-C target-feature=-crt-static"

RUN apk add --no-cache musl-dev openssl-dev pkgconfig

WORKDIR /app
COPY ./ /app

RUN cargo build --release
RUN strip target/release/backend

FROM alpine:3.21

RUN apk add --no-cache libgcc

COPY --from=0 /app/target/release/backend .
CMD ["/backend"]
