setup:
  git config core.hooksPath .githooks

run-source *FLAG:
  RUST_LOG=${RUST_LOG:-iris=debug} cargo r {{FLAG}} -- -c ./_ignored/source/iris.toml server

run-receiver *FLAG:
  RUST_LOG=${RUST_LOG:-iris=debug} cargo r {{FLAG}} -- -c ./_ignored/receiver/iris.toml server

new-migrate DESCRIPTION:
  diesel migration generate {{DESCRIPTION}}

generate-migrate SQLITE_PATH DESCRIPTION:
  diesel migration generate \
    --database-url "sqlite://{{SQLITE_PATH}}" \
    --diff-schema=src/infra/schema.rs \
    {{DESCRIPTION}}

run-migrate SQLITE_PATH:
  diesel migration run \
    --database-url "sqlite://{{SQLITE_PATH}}"
