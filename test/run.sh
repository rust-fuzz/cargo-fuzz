cargo install
PATH=$PATH:/home/travis/.cargo/bin
cd test/testcrate
cargo build
cargo-fuzz init
sed -i 's/\/\/.*/testcrate\:\:test_func\(data\)\;/g' fuzz/fuzzers/fuzzer_script_1.rs
cargo-fuzz run fuzzer_script_1

