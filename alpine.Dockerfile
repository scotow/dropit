FROM ekidd/rust-musl-builder:stable AS builder

COPY --chown=rust:rust . ./

RUN cargo build --release --bin dropit

# -------------------

FROM alpine:latest

COPY --from=builder /home/rust/src/target/x86_64-unknown-linux-musl/release/dropit /app/dropit

ENTRYPOINT ["/app/dropit"]