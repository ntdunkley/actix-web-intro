# Builder state
FROM rust:1.66 as builder

WORKDIR /app
RUN apt update && apt install lld clang -y
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release


# Runtime stage
FROM debian:bullseye-slim as runtime

WORKDIR /app

# Install OpenSSL - it is dynamically linked by some of our dependencies
# Install ca-certificates - it is needed to verify TLS certificates
# when establishing HTTPS connections
RUN apt update -y \
  && apt install -y --no-install-recommends openssl ca-certificates \
  # Clean up
  && apt autoremove -y \
  && apt clean -y \
  && rm -rf /var/lib/apt/lists/*

# Copy compiled binary from builder stage
COPY --from=builder /app/target/release/zero2prod zero2prod
# Copy config directory as we need the config files
COPY config config
ENV APP_ENVIRONMENT prod
ENTRYPOINT ["./zero2prod"]