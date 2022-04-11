FROM rust:1.60-slim AS builder

# Required by sqlx even if we don't use any SSL connection.
#RUN apt-get update && apt-get install -y openssl libssl-dev pkg-config

WORKDIR /app
COPY . .
RUN cargo build --release

#------------

FROM gcr.io/distroless/cc 

COPY --from=builder /app/target/release/dropit /dropit

ENTRYPOINT ["/dropit"]
