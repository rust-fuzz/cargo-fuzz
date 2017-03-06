#![no_main]
extern crate libfuzzer_sys;
extern crate testcrate;
#[export_name="rust_fuzzer_test_input"]
pub extern fn go(data: &[u8]) {
    testcrate::test_func(data);
}
