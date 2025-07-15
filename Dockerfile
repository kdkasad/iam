# UI builder
FROM docker.io/library/alpine:3.22 AS node-builder

# Install node
RUN apk update
RUN apk add pnpm

# Build UI
WORKDIR /src
COPY ui .
ARG PNPM_HOME=/var/lib/pnpm
RUN \
    --mount=type=cache,target=$PNPM_HOME \
    --mount=type=cache,target=node_modules \
    pnpm i && pnpm build


# Server builder
FROM docker.io/library/rust:1.88-alpine AS rust-builder

# Install build dependencies
RUN apk add build-base openssl-dev openssl-libs-static

# Build server
WORKDIR /src
COPY . .
RUN \
    --mount=type=cache,target=target \
    --mount=type=cache,target=$CARGO_HOME/git \
    --mount=type=cache,target=$CARGO_HOME/registry \
    cargo build -p iam-server --bin iam-server --release --locked --features sqlite3 && \
    # Copy executable out of the cache directory so it can be used in the final image
    cp target/release/iam-server .


# Assembled image
FROM docker.io/library/alpine:3.22
WORKDIR /app
COPY --from=node-builder /src/build /app/ui
COPY --from=rust-builder /src/iam-server /app/iam-server

VOLUME /db
ENV DB_BACKEND=sqlite3
ENV STATIC_DIR=/app/ui
ENV DB_PATH=/db/db.sqlite3
EXPOSE 3000
ENTRYPOINT ["/app/iam-server"]
