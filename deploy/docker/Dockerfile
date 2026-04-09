# =============================================================================
# LogBog — Multi-stage Docker build
# =============================================================================

# Stage 1: Build
FROM rust:1.83-slim-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests first for caching
COPY Cargo.toml Cargo.lock* ./
COPY crates/logbog-core/Cargo.toml crates/logbog-core/
COPY crates/logbog-cli/Cargo.toml crates/logbog-cli/
COPY crates/logbog-collector/Cargo.toml crates/logbog-collector/
COPY crates/logbog-parser/Cargo.toml crates/logbog-parser/
COPY crates/logbog-packs/Cargo.toml crates/logbog-packs/
COPY crates/logbog-engine/Cargo.toml crates/logbog-engine/
COPY crates/logbog-storage/Cargo.toml crates/logbog-storage/
COPY crates/logbog-api/Cargo.toml crates/logbog-api/

# Create dummy source files for dependency caching
RUN for crate in logbog-core logbog-cli logbog-collector logbog-parser logbog-packs logbog-engine logbog-storage logbog-api; do \
        mkdir -p "crates/$crate/src" && \
        echo "fn main() {}" > "crates/$crate/src/main.rs" && \
        echo "" > "crates/$crate/src/lib.rs"; \
    done

# Build dependencies only (cached layer)
RUN cargo build --release 2>/dev/null || true

# Copy actual source code
COPY crates/ crates/

# Build the real binary
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create logbog user
RUN useradd -r -s /bin/false -m -d /var/lib/logbog logbog

# Copy binary
COPY --from=builder /build/target/release/logbog /usr/local/bin/logbog

# Create directories
RUN mkdir -p /etc/logbog /var/lib/logbog/packs /var/log/logbog \
    && chown -R logbog:logbog /var/lib/logbog /var/log/logbog

# Default config
COPY deploy/docker/logbog.docker.toml /etc/logbog/logbog.toml

EXPOSE 6060 4317 4318 5514

USER logbog
WORKDIR /var/lib/logbog

ENTRYPOINT ["logbog"]
CMD ["start", "--foreground", "-c", "/etc/logbog/logbog.toml"]
