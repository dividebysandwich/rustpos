# Dockerfile
FROM rust:1.89 as builder

# Install cargo-leptos and wasm target
RUN cargo install cargo-leptos
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app

# Copy workspace and project files
COPY Cargo.toml ./
COPY common/Cargo.toml common/
COPY common/src common/src
COPY frontend/Cargo.toml frontend/
COPY frontend/style frontend/style
COPY frontend/assets frontend/assets
COPY frontend/locales frontend/locales

# Copy actual source code
COPY frontend/src frontend/src

# Build the actual application
RUN cargo leptos build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libudev1 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary and site files
COPY --from=builder /app/target/release/rustpos /app/rustpos
COPY --from=builder /app/site /app/site

# Copy data assets
RUN mkdir -p /app/data
COPY --from=builder /app/frontend/assets/logo_receipt.png /app/data/

EXPOSE 3000

CMD ["./rustpos"]
