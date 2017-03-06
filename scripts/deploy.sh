cd etc/testcrate/

cargo-fuzz init

sed -i 's/\/\/.*/testcrate\:\:test_func\(data\)\;/g' fuzz/fuzzers/fuzzer_script_1.rs

cargo-fuzz run fuzzer_script_1

