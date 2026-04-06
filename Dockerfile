# Dockerfile
FROM rust:1.89 as builder

# Install cargo-leptos and wasm target
RUN cargo install cargo-leptos
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app

# Copy workspace and project files
COPY Cargo.toml ./
COPY frontend/Cargo.toml frontend/
COPY frontend/style frontend/style
COPY frontend/assets frontend/assets

# Build dependencies first (for better caching)
RUN mkdir -p frontend/src && echo "pub mod app; #[cfg(feature=\"ssr\")] pub mod printer;" > frontend/src/lib.rs
RUN echo "pub fn App() -> impl leptos::IntoView {}" > frontend/src/app.rs
RUN echo "" > frontend/src/printer.rs
RUN echo "#[cfg(feature=\"ssr\")] fn main() {} #[cfg(not(feature=\"ssr\"))] fn main() {}" > frontend/src/main.rs
RUN cd frontend && cargo leptos build --release 2>/dev/null || true

# Copy actual source code
COPY frontend/src frontend/src

# Build the actual application
RUN cd frontend && cargo leptos build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary and site files
COPY --from=builder /app/target/server/release/rustpos /app/rustpos
COPY --from=builder /app/frontend/site /app/site

# Copy data assets
RUN mkdir -p /app/data
COPY --from=builder /app/frontend/assets/logo_receipt.png /app/data/

EXPOSE 3000

CMD ["./rustpos"]
