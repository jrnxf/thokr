FROM rust:latest as builder

RUN USER=root

RUN mkdir thokr
WORKDIR /thokr
COPY . ./


RUN rustup target add x86_64-unknown-linux-musl 
RUN apt update 
RUN apt -y install musl-tools musl-dev
RUN apt-get install -y build-essential
RUN yes | apt install gcc-x86-64-linux-gnu

ENV RUSTFLAGS='-C linker=x86_64-linux-gnu-gcc'
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM debian:buster-slim

ARG APP=/usr/src/app

ENV APP_USER=appuser

RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP}

# Copy the compiled binaries into the new container.
COPY --from=builder /thokr/target/x86_64-unknown-linux-musl/release/thokr ${APP}/thokr

RUN chown -R $APP_USER:$APP_USER ${APP}
USER $APP_USER
WORKDIR ${APP}

ENTRYPOINT ["./thokr"]
