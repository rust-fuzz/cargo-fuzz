// Copyright 2016 rust-fuzz developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

extern crate cargo_metadata;
extern crate clap;
extern crate rustc_serialize;
extern crate term;

use cargo_metadata::{metadata, Package};
use clap::{App, Arg, SubCommand, ArgMatches, AppSettings};
use std::{env, error, fs, io, path, process};
use std::io::Write;

mod utils;

fn main() {
    let app = App::new("cargo-fuzz")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0"))
        .about(option_env!("CARGO_PKG_DESCRIPTION").unwrap_or(""))
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::GlobalVersion)
        // cargo passes in the subcommand name to the invoked executable. Use a hidden, optional
        // positional argument to deal with it?
        .arg(Arg::with_name("dummy").possible_value("fuzz").required(false).hidden(true))
        .subcommand(SubCommand::with_name("init").about("Initialize the fuzz folder"))
        .subcommand(SubCommand::with_name("run").about("Run the fuzz target in fuzz/fuzzers")
            .setting(AppSettings::TrailingVarArg)
            .arg(Arg::with_name("TARGET").required(true))
            .arg(Arg::with_name("ARGS").multiple(true))
        )
        .subcommand(SubCommand::with_name("add").about("Add a new fuzz target")
                    .arg(Arg::with_name("TARGET").required(true)))
        .subcommand(SubCommand::with_name("list").about("List all fuzz targets"));
    let args = app.get_matches();

    ::std::process::exit(match args.subcommand() {
        ("init", _) => init_fuzz(),
        ("add", matches) => add_target(matches.expect("arguments present")),
        ("list", _) => list_targets(),
        ("run", matches) => exec_target(matches.expect("arguments present")),
        (s, _) => panic!("unimplemented subcommand {}!", s),
    }.map(|_| 0).unwrap_or_else(|err| {
        writeln!(io::stderr(), "Error: {}", err).expect("failed writing to stderr");
        1
    }));
}

/// Create all the files and folders we need to run
///
/// This will not clone libfuzzer-sys
fn init_fuzz() -> Result<(), Box<error::Error>> {
    let me = get_package();

    fs::create_dir("./fuzz")?;
    fs::create_dir("./fuzz/fuzzers")?;

    let mut cargo = fs::File::create(path::Path::new("./fuzz/Cargo.toml"))?;

write!(cargo, r#"
[package]
name = "{0}-fuzz"
version = "0.0.1"
authors = ["Automatically generated"]
publish = false

[dependencies.{0}]
path = ".."

# Prevent this from interfering with workspaces
[workspace]
members = ["."]

[[bin]]
name = "fuzzer_script_1"
path = "fuzzers/fuzzer_script_1.rs"
"#, me.name)?;

    let mut ignore = fs::File::create(path::Path::new("./fuzz/.gitignore"))?;

write!(ignore, r#"
target
libfuzzer
corpus
artifacts
"#)?;

    let mut script = fs::File::create(path::Path::new("./fuzz/fuzzers/fuzzer_script_1.rs"))?;
    dummy_target(&mut script, &me)
}


fn list_targets() -> Result<(), Box<error::Error>> {
    if !path::Path::new("./fuzz").is_dir() {
        return Err("Fuzzing has not been initialized. Run `cargo fuzz init` first.".into());
    }
    env::set_current_dir("./fuzz")?;
    let package = get_package();
    for target in &package.targets {
        println!("{}", target.name);
    }
    Ok(())
}

/// Returns metadata for the Cargo package in the current directory
fn get_package() -> Package {
    // todo error handling
    let meta = metadata(None).unwrap();
    let mut p = env::current_dir().unwrap();
    p.push("Cargo.toml");
    let p = p.to_str().unwrap();
    meta.packages.into_iter().find(|package| package.manifest_path == p).unwrap()
}

/// If the package contains a library target, generate an `extern crate` line to link to it.
fn link_to_lib(pkg: &Package) -> Option<String> {
    pkg.targets.iter()
               .find(|target| target.kind.iter().any(|k| k == "lib"))
               .map(|target| format!("extern crate {};\n", target.name.replace("-", "_")))
}

/// Create a dummy fuzz target script at the given path
fn dummy_target(script: &mut fs::File, pkg: &Package) -> Result<(), Box<error::Error>> {
write!(script, r#"#![no_main]
extern crate libfuzzer_sys;
{}
#[export_name="rust_fuzzer_test_input"]
pub extern fn go(data: &[u8]) {{
    // fuzzed code goes here
}}"#, link_to_lib(pkg).unwrap_or(String::new())).map_err(|e| e.into())
}

/// Add a new fuzz target script with a given name
fn add_target<'a>(args: &ArgMatches<'a>) -> Result<(), Box<error::Error>> {
    let target: String = args.value_of_os("TARGET").expect("TARGET is required").to_os_string()
        .into_string().map_err(|_| "TARGET must be valid unicode")?;
    let components: &[&path::Path] = &["fuzz".as_ref(), "fuzzers".as_ref(), target.as_ref()];
    let target_file = components.iter().collect::<path::PathBuf>();
    let mut script = fs::File::create(path::Path::new(&target_file))?;
    let me = get_package();
    dummy_target(&mut script, &me)?;

    let mut cargo = fs::OpenOptions::new().append(true).open(path::Path::new("./fuzz/Cargo.toml"))?;

write!(cargo, r#"
[[bin]]
name = "{0}"
path = "fuzzers/{0}.rs"
"#, target).map_err(|e| e.into())

}

/// Build or rebuild libFuzzer (rebuilds only if the compiler version changed)
///
/// We can't just use libFuzzer as a dependency since libgcc will
/// get compiled with sanitizer support. RUSTFLAGS does not discriminate
/// between build dependencies and regular ones.
///
/// https://github.com/rust-lang/cargo/issues/3739
fn rebuild_libfuzzer() -> Result<(), Box<error::Error>> {
    if let Err(_) = env::set_current_dir("./libfuzzer") {
        let mut git = process::Command::new("git");
        let mut cmd = git.arg("clone")
                         .arg("https://github.com/rust-fuzz/libfuzzer-sys.git")
                         .arg("libfuzzer");
        let result = cmd.spawn()?.wait()?;
        if !result.success() {
            return Err("Failed to clone libfuzzer-sys".into())
        }
        env::set_current_dir("./libfuzzer")?;
    }
    let mut cmd = process::Command::new("cargo");
    cmd.arg("build")
       .arg("--release")
       .spawn()?
       .wait()?;

    let result = cmd.spawn()?.wait()?;
    if !result.success() {
        return Err("Failed to build libfuzzer-sys".into())
    }
    env::set_current_dir("..")
        .map_err(|e| e.into())
}

fn make_dir_if_not_exist(dir: &str) -> Result<(), io::Error> {
    if let Err(k) = fs::create_dir(dir) {
        if k.kind() == io::ErrorKind::AlreadyExists {
            // do nothing
        } else {
            return Err(k);
        }
    }
    Ok(())
}
/// Fuzz a given fuzz target
fn exec_target<'a>(args: &ArgMatches<'a>) -> Result<(), Box<error::Error>> {
    let target: String = args.value_of_os("TARGET").expect("TARGET is required").to_os_string()
        .into_string().map_err(|_| "TARGET must be valid unicode")?;
    let exec_args = args.values_of_os("ARGS");

    let target_triple = "x86_64-unknown-linux-gnu";

    env::set_current_dir("./fuzz")?;
    rebuild_libfuzzer()?;
    let mut flags = env::var("RUSTFLAGS").unwrap_or("".into());
    if !flags.is_empty() {
        flags.push(' ');
    }
    flags.push_str("-Cpasses=sancov -Cllvm-args=-sanitizer-coverage-level=3 -Zsanitizer=address -Cpanic=abort");
    let mut cmd = process::Command::new("cargo");
    cmd.arg("rustc")
       .arg("--verbose")
       .arg("--bin")
       .arg(&target)
       .arg("--target")
       .arg(target_triple) // won't pass rustflags to build scripts
       .arg("--")
       .arg("-L")
       .arg("libfuzzer/target/release")
       .env("RUSTFLAGS", &flags);

    let result = cmd.spawn()?.wait()?;
    if !result.success() {
        return Err("Failed to build fuzz target".into())
    }

    make_dir_if_not_exist("corpus")?;
    make_dir_if_not_exist("artifacts")?;

    // can't use cargo run since we can't pass -L args to it
    let components: &[&path::Path] = &["target".as_ref(),
                                       target_triple.as_ref(),
                                       "debug".as_ref(),
                                       target.as_ref()];
    let mut run_cmd = process::Command::new(components.iter().collect::<path::PathBuf>());
    run_cmd.arg("-artifact_prefix=artifacts/")
           .env("ASAN_OPTIONS", "detect_odr_violation=0");
    exec_args.map(|args| for arg in args { run_cmd.arg(arg); });
    run_cmd.arg("corpus"); // must be last arg
    exec_cmd(&mut run_cmd)?;
    Ok(())
}

#[cfg(unix)]
fn exec_cmd(cmd: &mut process::Command) -> ::std::io::Result<process::ExitStatus> {
    use std::os::unix::process::CommandExt;
    Err(cmd.exec())
}

#[cfg(not(unix))]
fn exec_cmd(cmd: &mut process::Command) -> ::std::io::Result<process::ExitStatus> {
    cmd.status()
}
