FROM rust:1.76-slim AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

#------------

FROM gcr.io/distroless/cc 

COPY --from=builder /app/target/release/dropit /dropit

ENTRYPOINT ["/dropit"]
