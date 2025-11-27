# Define app name as a build argument
ARG APP_NAME=blendwerk

FROM clux/muslrust AS system
ARG APP_NAME

RUN apt-get update && apt-get -y upgrade

FROM system AS builder-dependencies

WORKDIR /source
COPY Cargo.toml Cargo.lock ./
COPY src src/

FROM builder-dependencies AS builder
ARG APP_NAME

WORKDIR /source

# Run tests (essentially mainly building as well)
RUN RUSTFLAGS="-C target-feature=+crt-static" cargo test --release

# Build for release with static linking
RUN RUSTFLAGS="-C target-feature=+crt-static" cargo build --release

RUN mkdir /app && \
    find ./target -iname "${APP_NAME}" -type f -exec cp {} /app \;

# Final stage
FROM scratch
ARG APP_NAME

# Copy the statically-linked binary
COPY --from=builder /app/${APP_NAME} /${APP_NAME}


ENTRYPOINT ["/blendwerk"]
