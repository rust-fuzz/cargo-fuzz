// Copyright 2016 rust-fuzz Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

extern crate cargo_metadata;
extern crate docopt;
extern crate rustc_serialize;

use cargo_metadata::metadata;
use docopt::Docopt;
use std::{env, fs, io, path, process};
use std::io::Write;

const USAGE: &'static str = "
Cargo Fuzz

Usage:
  cargo fuzz --init
  cargo fuzz --target TARGET
  cargo fuzz --add TARGET
  cargo fuzz (-h | --help)

Options:
  -h --help         Show this screen.
  --init            Initialize fuzz folder
  --target TARGET   Run with given fuzz target in fuzz/fuzzers
  --add TARGET      Add a new fuzz target
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_init: bool,
    flag_add: Option<String>,
    flag_target: Option<String>,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let result = if args.flag_init {
        init_fuzz()
    } else if let Some(target) = args.flag_add {
        add_target(target)
    } else if let Some(target) = args.flag_target {
        run_target(target)
    } else {
        println!("Invalid arguments. Usage:\n{}", USAGE);
        return;
    };
    if let Err(error) = result {
        println!("Error: {:?}", error);
    }
}

fn init_fuzz() -> io::Result<()> {
    // todo error handling
    let meta = metadata(None).unwrap();
    let mut p = env::current_dir().unwrap();
    p.push("Cargo.toml");
    let p = p.to_str().unwrap();
    let me = meta.packages.iter().find(|package| package.manifest_path == p).unwrap();

    fs::create_dir("./fuzz")?;
    fs::create_dir("./fuzz/fuzzers")?;

    let mut cargo = fs::File::create(path::Path::new("./fuzz/Cargo.toml"))?;

write!(cargo, r#"
[package]
name = "{0}-fuzz"
version = "0.0.1"
authors = ["Automatically generated"]

[build]
rustflags = "-Cpasses=sancov -Cllvm-args=-sanitizer-coverage-level=3 -Zsanitizer=address -Cpanic=abort"

[dependencies.{0}]
path = ".."

[dependencies.fuzzer-sys]
git = "https://github.com/rust-fuzz/libfuzzer-sys.git"

[[bin]]
name = "fuzzer_script_1"
path = "fuzzers/fuzzer_script_1.rs"
"#, me.name)?;

    let mut script = fs::File::create(path::Path::new("./fuzz/fuzzers/fuzzer_script_1.rs"))?;
    dummy_target(&mut script)
}

fn dummy_target(script: &mut fs::File) -> io::Result<()> {
write!(script, r#"
#![no_main]
extern crate fuzzer_sys;

#[export_name="LLVMFuzzerTestOneInput"]
pub extern fn go(data: *const u8, size: isize) -> i32 {{
    // fuzzed code goes here
    0
}}"#)
}

fn add_target(target: String) -> io::Result<()> {
    let target_file = format!("fuzz/fuzzers/{}.rs", target);
    let mut script = fs::File::create(path::Path::new(&target_file))?;
    dummy_target(&mut script)?;

    let mut cargo = fs::OpenOptions::new().append(true).open(path::Path::new("./fuzz/Cargo.toml"))?;

write!(cargo, r#"
[[bin]]
name = "{0}"
path = "fuzzers/{0}.rs"
"#, target)

}

fn run_target(target: String) -> io::Result<()> {
    env::set_current_dir("./fuzz")?;
    let mut cmd = process::Command::new("cargo");
    cmd.arg("run")
       .arg("--verbose")
       .arg("--bin")
       .arg(&target)
    cmd.spawn()?.wait()?;
    Ok(())
}
