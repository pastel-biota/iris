setup:
  git config core.hooksPath .githooks

run-source:
  RUST_LOG=${RUST_LOG:-iris=debug} cargo r --release -- -c ./_ignored/source/iris.toml server
                                                     
run-receiver:                                        
  RUST_LOG=${RUST_LOG:-iris=debug} cargo r --release -- -c ./_ignored/receiver/iris.toml server

