# Build stage
FROM rust:1.78-slim-bookworm as builder

WORKDIR /usr/src/webgone
COPY . .

# Build the application in release mode
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Create a directory for the database
RUN mkdir -p /data

WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /usr/src/webgone/target/release/webgone .

# Create a volume for persistent storage
VOLUME ["/data"]

# Set the working directory to where the database will be stored
WORKDIR /data

# Run the binary
ENTRYPOINT ["/app/webgone"]
