FROM rust:1.94.0-alpine AS builder
WORKDIR /app
RUN mkdir src && echo "fn main() { println!(\"Hi!\"); }" > ./src/main.rs
COPY Cargo.toml Cargo.lock .
RUN cargo build --release;
COPY . .
RUN touch src/main.rs && cargo build --release;

FROM alpine
WORKDIR /app
COPY --from=builder /app/target/release/iris ./iris
RUN adduser -D runner && chown runner:runner ./iris;
USER runner
CMD ["/app/iris"]
