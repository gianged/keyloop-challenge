FROM rust:1-slim-trixie AS build
WORKDIR /app
ENV SQLX_OFFLINE=true

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY .sqlx ./.sqlx
COPY src ./src
RUN touch src/main.rs && cargo build --release

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home app

COPY --from=build /app/target/release/keyloop-challenge /usr/local/bin/keyloop-challenge
USER app
EXPOSE 8080
CMD ["keyloop-challenge"]
