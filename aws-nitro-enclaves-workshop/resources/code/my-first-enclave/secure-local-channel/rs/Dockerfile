# Build Stage
FROM public.ecr.aws/amazonlinux/amazonlinux:2023 AS builder

# Install build dependencies
RUN dnf install -y gcc openssl-devel

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /usr/src/app
# Copy the source code selectively (don't include client.rs inside Docker)
COPY Cargo.toml ./
RUN mkdir src && mkdir src/bin
COPY src/bin/server.rs src/bin/
COPY src/bin/traffic_forwarder.rs src/bin/

# Build the release binaries (client is run locally, not in the enclave)
RUN cargo build --release --bin server --bin traffic_forwarder

# Run Stage
# vsock demo image
FROM public.ecr.aws/amazonlinux/amazonlinux:minimal

# Install iproute for modifying network config, and openssl runtime for reqwest
RUN dnf install iproute openssl -y

WORKDIR /app

# Copy built binaries from the builder stage
COPY --from=builder /usr/src/app/target/release/server ./
COPY --from=builder /usr/src/app/target/release/traffic_forwarder ./
COPY run.sh ./

RUN chmod +x /app/run.sh

CMD ["/app/run.sh"]
