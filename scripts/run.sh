#!/usr/bin/env bash

set -eux

cd "$(dirname $0)/.."

CARGO_TOML="$(pwd)/Cargo.toml"
function cargo_fuzz() {
    cargo run --manifest-path "$CARGO_TOML" -- $@
}

cd ./testcrate
cargo build

if [[ -d ./fuzz ]]; then
    rm -r ./fuzz
fi

cargo_fuzz init

# First, run a fuzz target that should find a crash.

# Replace the `// fuzzed code goes here` comment with a call to our test crate's
# function.
sed -i'' -e 's/\/\/.*/testcrate\:\:test_func\(data\)\;/g' fuzz/fuzz_targets/fuzz_target_1.rs

if cargo_fuzz run fuzz_target_1 -- -runs=1000; then
    echo "Error: Fuzzing the test crate should not succeed!"
    exit 100
fi

# There should be something here! `ls` will exit non-zero if not.
ls fuzz/artifacts/fuzz_target_1/crash-*

# Second, run a fuzz target that should _not_ crash.
cargo_fuzz add fuzz_target_2
cargo_fuzz run fuzz_target_2 -- -runs=1000

echo "OK! All tests passed!"
