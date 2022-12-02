FROM rust AS chef
WORKDIR /app
RUN cargo install cargo-chef

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
RUN apt update && \
    apt install --no-install-recommends -y protobuf-compiler
COPY . .
RUN cargo build --bin chat-server --release

FROM debian:bullseye-slim AS runtime
COPY --from=builder /app/target/release/chat-server /usr/local/bin/chat-server
EXPOSE 80
ENV PORT=80
CMD ["chat-server"]
