FROM debian:bookworm-slim
WORKDIR /app
COPY target/release/iris ./iris
RUN adduser --disabled-password --gecos "" runner && chown runner:runner ./iris
USER runner
CMD ["/app/iris"]
