cargo build --release
hyperfine --warmup 8 --min-runs 256 'target/release/capra-singleplanner < samples/sample_rev.json'