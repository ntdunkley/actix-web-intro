# cargo chef stage
FROM lukemathwalker/cargo-chef:latest-rust-1.66 as chef
WORKDIR /app
RUN apt update && apt install lld clang -y

# cargo chef prepare stage
FROM chef as planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare --recipe-path recipe.json

# builder stage
FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# build our project dependencies, not our application
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE true
# build our application
RUN cargo build --release --bin zero2prod


# runtime stage
FROM debian:bullseye-slim as runtime

WORKDIR /app

# install OpenSSL - it is dynamically linked by some of our dependencies
# install ca-certificates - it is needed to verify TLS certificates when establishing HTTPS connections
RUN apt update -y \
  && apt install -y --no-install-recommends openssl ca-certificates \
  # Clean up
  && apt autoremove -y \
  && apt clean -y \
  && rm -rf /var/lib/apt/lists/*

# copy compiled binary from builder stage
COPY --from=builder /app/target/release/zero2prod zero2prod
# copy config directory as we need the config files
COPY config config
ENV APP_ENVIRONMENT prod
ENTRYPOINT ["./zero2prod"]