cargo install --force
PATH=$PATH:/home/travis/.cargo/bin
cd testcrate
cargo build
cargo-fuzz init
sed -i'' -e 's/\/\/.*/testcrate\:\:test_func\(data\)\;/g' fuzz/fuzzers/fuzzer_script_1.rs

if cargo-fuzz run fuzzer_script_1 -- -runs=1000; then
    exit 100; # Should not succeed!
else
    :;
fi

# should be something there!
ls fuzz/artifacts/fuzzer_script_1/crash-*

