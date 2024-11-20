# Stage 1: Build the application
FROM rust:1.82.0 as builder

# Set environment variables
ENV SQLX_OFFLINE true

# Create a new empty shell project
RUN USER=root cargo new --bin myapp
WORKDIR /myapp

# Copy the Cargo.toml and Cargo.lock and download the dependencies
COPY Cargo.toml Cargo.lock ./
COPY ./src ./src

# Build the dependencies
RUN cargo build --release

# Stage 2: Run the application
FROM debian:bullseye-slim

# Install dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the build artifact from the builder stage
COPY --from=builder /myapp/target/release/myapp /usr/local/bin/myapp

# Copy the environment variables file
COPY .env /myapp/.env

# Set the working directory
WORKDIR /myapp

# Expose the application port (example: 8080)
EXPOSE 8080

# Run the binary
CMD ["myapp"]