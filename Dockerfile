# Build stage
FROM --platform=$BUILDPLATFORM rust:1.78-slim-bookworm as builder

# Install cross-compilation tools
RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    apt-get install -y --no-install-recommends \
    g++-aarch64-linux-gnu \
    libc6-dev-arm64-cross \
    pkg-config && \
    rm -rf /var/lib/apt/lists/*

# Add target support for ARM64
RUN rustup target add aarch64-unknown-linux-gnu

WORKDIR /usr/src/webgone
COPY . .

# Build for the target architecture
ARG TARGETARCH
RUN case "$TARGETARCH" in \
        "arm64") \
            echo "Building for ARM64..." && \
            export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc && \
            export PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig && \
            cargo build --release --target aarch64-unknown-linux-gnu && \
            cp target/aarch64-unknown-linux-gnu/release/webgone /usr/src/webgone/webgone \
            ;; \
        *) \
            echo "Building for AMD64..." && \
            cargo build --release && \
            cp target/release/webgone /usr/src/webgone/webgone \
            ;; \
    esac

# Runtime stage
FROM --platform=$TARGETPLATFORM debian:bookworm-slim

# Create a directory for the database
RUN mkdir -p /data

WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /usr/src/webgone/webgone .

# Create a volume for persistent storage
VOLUME ["/data"]

# Set the working directory to where the database will be stored
WORKDIR /data

# Run the binary
ENTRYPOINT ["/app/webgone"]

CMD [ "watch" ]
