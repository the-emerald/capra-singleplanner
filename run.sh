set -ex
export RUST_BACKTRACE="full"

# shellcheck disable=SC2002
#cat src/sample_46_15.json | cargo run 2> output_sample_40_22.txt
cargo run --release < sample_sammy.json