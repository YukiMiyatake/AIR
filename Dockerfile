# syntax=docker/dockerfile:1

# AIR development image: Rust (production airc) + Node (Phase 1 TS bootstrap).
FROM rust:1.93-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    pkg-config \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Node 22 (for tools/airc TypeScript bootstrap)
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# Pre-warm cargo registry path ownership for non-root later if needed
ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

CMD ["bash"]
