FROM ubuntu:24.04
WORKDIR /app
COPY iris ./iris
RUN groupadd --gid 10001 runner \
    && useradd --uid 10001 --gid runner --create-home runner \
    && chown runner:runner ./iris
USER runner
CMD ["/app/iris"]
