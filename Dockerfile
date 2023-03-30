FROM rust:1.63-slim AS builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin dropit-server

#------------

FROM gcr.io/distroless/cc 

COPY --from=builder /app/target/release/dropit-server /dropitd

ENTRYPOINT ["/dropitd"]
