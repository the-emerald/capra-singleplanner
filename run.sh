set -ex
export RUST_BACKTRACE="full"
cargo run < src/sample_sammy.json
