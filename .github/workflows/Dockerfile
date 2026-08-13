FROM ubuntu:24.04
WORKDIR /app
COPY iris ./iris
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 runner \
    && useradd --uid 10001 --gid runner --create-home runner \
    && chown runner:runner ./iris
USER runner
CMD ["/app/iris"]
