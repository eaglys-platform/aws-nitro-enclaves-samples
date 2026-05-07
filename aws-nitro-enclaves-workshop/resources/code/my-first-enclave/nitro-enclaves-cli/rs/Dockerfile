# // Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# // SPDX-License-Identifier: MIT-0

FROM public.ecr.aws/amazonlinux/amazonlinux:2023 AS builder

# Install rust compiler and gcc
RUN dnf install gcc rust -y

WORKDIR /app
COPY server.rs ./

# Compile the server
RUN rustc server.rs -C opt-level=3

FROM public.ecr.aws/amazonlinux/amazonlinux:minimal

WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/server ./

CMD ["/app/server"]
