FROM rust:slim-bookworm AS builder

WORKDIR /thokr

COPY . ./

# Build a normal glibc release binary for the NATIVE arch (amd64 in CI, arm64
# locally). No cross toolchain, no musl: bookworm's glibc matches the runtime
# stage (debian:bookworm-slim), so the dynamically linked binary is compatible.
RUN cargo build --release \
    && cp target/release/thokr /thokr/thokr-bin

FROM debian:bookworm-slim

ARG APP=/usr/src/app

ENV APP_USER=appuser

RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP}

COPY --from=builder /thokr/thokr-bin ${APP}/thokr

RUN chown -R $APP_USER:$APP_USER ${APP}
USER $APP_USER
WORKDIR ${APP}

ENTRYPOINT ["./thokr"]
