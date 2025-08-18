# Dockerfile
FROM rust:1.89 as builder

# Install trunk and wasm target
RUN cargo install trunk
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app

# Copy workspace files
COPY Cargo.toml ./
COPY backend/Cargo.toml backend/
COPY frontend/Cargo.toml frontend/
COPY frontend/Trunk.toml frontend/
COPY frontend/index.html frontend/

# Build dependencies first (for better caching)
RUN mkdir backend/src && echo "fn main() {}" > backend/src/main.rs
RUN mkdir frontend/src && echo "fn main() {}" > frontend/src/main.rs
RUN cd backend && cargo build --release
RUN cd frontend && cargo build --release --target wasm32-unknown-unknown

# Copy actual source code
COPY backend/src backend/src
COPY frontend/src frontend/src
COPY frontend/style frontend/style

# Build the actual application
RUN cd frontend && trunk build --release
RUN cd backend && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary
COPY --from=builder /app/target/release/rustpos-backend /app/
# Copy the static files
COPY --from=builder /app/frontend/dist /app/static

RUN mkdir -p /app/data

EXPOSE 3000

CMD ["./pos-backend"]
