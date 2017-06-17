cargo install --force
PATH=$PATH:/home/travis/.cargo/bin
cd testcrate
cargo build
cargo-fuzz init
sed -i'' -e 's/\/\/.*/testcrate\:\:test_func\(data\)\;/g' fuzz/fuzz_targets/fuzz_target_1.rs

if cargo-fuzz run fuzz_target_1 -- -runs=1000; then
    exit 100; # Should not succeed!
else
    :;
fi

# should be something there!
ls fuzz/artifacts/fuzz_target_1/crash-*

