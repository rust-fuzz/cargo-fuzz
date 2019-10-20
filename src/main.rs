// Copyright 2016 rust-fuzz developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use clap::{App, Arg, SubCommand, ArgMatches, AppSettings};
use std::{env, fs, path, ffi, process};
use std::io::Write;
use std::io::Read;

#[macro_use]
mod templates;
mod utils;

error_chain::error_chain! {
    foreign_links {
        Toml(toml::de::Error);
        Io(::std::io::Error);
    }
}

static FUZZ_TARGETS_DIR_OLD: &'static str = "fuzzers";
static FUZZ_TARGETS_DIR: &'static str = "fuzz_targets";

// clap's long_about() makes `cargo fuzz --help` unreadable,
// and clap's before_help() injects our long about text before the version,
// so change the default template slightly.
const LONG_ABOUT_TEMPLATE: &'static str =
"{bin} {version}
{about}

USAGE:
    {usage}

{before-help}

{all-args}

{after-help}";

fn main() {
    let app = App::new("cargo-fuzz")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0"))
        .about(option_env!("CARGO_PKG_DESCRIPTION").unwrap_or(""))
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::DeriveDisplayOrder)
        // cargo passes in the subcommand name to the invoked executable. Use a hidden, optional
        // positional argument to deal with it?
        .arg(Arg::with_name("dummy")
             .possible_value("fuzz")
             .required(false)
             .hidden(true))
        .subcommand(SubCommand::with_name("init")
            .about("Initialize the fuzz folder")
            .arg(Arg::with_name("target")
                 .long("target").short("t")
                 .required(false)
                 .default_value("fuzz_target_1")
                 .help("Name of the first fuzz target to create")))
        .subcommand(fuzz_subcommand("run")
            .template(LONG_ABOUT_TEMPLATE)
            .about("Run a fuzz target")
            .before_help(
"The fuzz target name is the same as the name of the fuzz target script \
in fuzz/fuzz_targets/, i.e. the name picked when running `cargo fuzz add`.

This will run the script inside the fuzz target with varying inputs \
until it finds a crash, at which point it will save the crash input \
to the artifact directory, print some output, and exit. Unless you \
configure it otherwise (see libFuzzer options below), this will run \
indefinitely.")
            .arg(Arg::with_name("CORPUS")
                 .multiple(true)
                 .help("Custom corpus directory or artifact files"))
            .arg(Arg::with_name("JOBS")
                 .long("jobs").short("j")
                 .takes_value(true)
                 .default_value("1")
                 .help("Number of concurrent jobs to run")
                 .validator(|v| Err(From::from(match v.parse::<u16>() {
                     Ok(0) => "0 jobs?",
                     Err(_) => "must be a valid integer representing a sane number of jobs",
                     _ => return Ok(()),
                 }))))
            .arg(Arg::with_name("ARGS")
                 .multiple(true)
                 .last(true)
                 .help("Additional libFuzzer arguments passed to the binary"))
            .after_help(
"A full list of libFuzzer options can be found at http://llvm.org/docs/LibFuzzer.html#options
You can also get this by running `cargo fuzz run fuzz_target -- -help=1`

Some useful options (to be used as `cargo fuzz run fuzz_target -- <options>`) include:
 - `-max_len=<len>`: Will limit the length of the input string to `<len>`
 - `-runs=<number>`: Will limit the number of tries (runs) before it gives up
 - `-max_total_time=<time>`: Will limit the amount of time to fuzz before it gives up
 - `-timeout=<time>`: Will limit the amount of time for a single run before it considers that run a failure
 - `-only_ascii`: Only provide ASCII input
 - `-dict=<file>`: Use a keyword dictionary from specified file. See http://llvm.org/docs/LibFuzzer.html#dictionaries")
        )
        .subcommand(fuzz_subcommand("cmin")
             .about("Corpus minifier")
             .arg(Arg::with_name("CORPUS")
                  .help("directory with corpus to minify"))
        )
        .subcommand(fuzz_subcommand("tmin")
             .about("Test case minifier")
             .arg(Arg::with_name("runs").long("runs")
                  .help("Number of attempts to minimize we should make")
                  .takes_value(true)
                  .default_value("255")
                  .validator(|v| Err(From::from(match v.parse::<u32>() {
                      Ok(0) => "0 jobs?",
                      Err(_) => "must be a valid integer representing a sane number of jobs",
                      _ => return Ok(()),
                  }))))
             .arg(Arg::with_name("CRASH")
                  .required(true)
                  .help("Crashing test case to minimize"))
        )
        .subcommand(SubCommand::with_name("add").about("Add a new fuzz target")
                    .arg(Arg::with_name("TARGET").required(true)
                         .help("Name of the fuzz target"))
        )
        .subcommand(SubCommand::with_name("list").about("List all fuzz targets"));
    let args = app.get_matches();

    process::exit(match args.subcommand() {
        ("init", matches) => FuzzProject::init(matches.expect("arguments present")).map(|_| ()),
        ("add", matches) => FuzzProject::new()
            .and_then(|p| p.add_target(matches.expect("arguments present"))),
        ("list", _) => FuzzProject::new()
            .and_then(|p| p.list_targets()),
        ("run", matches) => FuzzProject::new()
            .and_then(|p| p.exec_fuzz(matches.expect("arguments present"))),
        ("cmin", matches) => FuzzProject::new()
            .and_then(|p| p.exec_cmin(matches.expect("arguments present"))),
        ("tmin", matches) => FuzzProject::new()
            .and_then(|p| p.exec_tmin(matches.expect("arguments present"))),
        (s, _) => panic!("unimplemented subcommand {}!", s),
    }.map(|_| 0).unwrap_or_else(|err| {
        utils::report_error(&err);
        1
    }));
}

fn fuzz_subcommand(name: &str) -> App {
    SubCommand::with_name(name)
        .setting(AppSettings::DeriveDisplayOrder)
        .arg(Arg::with_name("release")
             .long("release").short("O")
             .help("Build artifacts in release mode, with optimizations"))
        .arg(Arg::with_name("debug_assertions")
             .long("debug-assertions").short("a")
             .help("Build artifacts with debug assertions enabled (default if not -O)"))
        .arg(Arg::with_name("no_default_features")
             .long("no-default-features")
             .help("Build artifacts with default Cargo features disabled"))
        .arg(Arg::with_name("all_features")
             .long("all-features")
             .help("Build artifacts with all Cargo features enabled"))
        .arg(Arg::with_name("features")
             .long("features")
             .takes_value(true)
             .help("Build artifacts with given Cargo feature enabled"))
        .arg(Arg::with_name("sanitizer")
             .long("sanitizer").short("s")
             .takes_value(true)
             .possible_values(&["address", "leak", "memory", "thread"])
             .default_value("address")
             .help("Use different sanitizer"))
        .arg(Arg::with_name("TRIPLE")
             .long("target")
             .default_value(utils::default_target())
             .help("Target triple of the fuzz target"))
        .arg(Arg::with_name("TARGET").required(true)
             .help("Name of the fuzz target"))
}

fn get_target(args: &ArgMatches) -> Result<String> {
    Ok(args.value_of_os("TARGET").expect("TARGET is required")
       .to_os_string().into_string()
       .map_err(|_| "TARGET must be valid unicode")?)
}

struct FuzzProject {
    /// Path to the root cargo project
    ///
    /// Not the project with fuzz targets, but the project being fuzzed
    root_project: path::PathBuf,
    targets: Vec<String>,
}

impl FuzzProject {
    fn new() -> Result<Self> {
        let mut project = FuzzProject {
            root_project: find_package()?,
            targets: Vec::new()
        };
        let manifest = project.manifest()?;
        if !is_fuzz_manifest(&manifest) {
            return Err(format!("manifest `{:?}` does not look like a cargo-fuzz manifest. \
                                Add following lines to override:\n\
                                [package.metadata]\ncargo-fuzz = true",
                                project.manifest_path()).into());
        }
        project.targets = collect_targets(&manifest);
        Ok(project)
    }

    /// Create the fuzz project structure
    ///
    /// This will not clone libfuzzer-sys
    fn init(args: &ArgMatches) -> Result<Self> {
        let project = FuzzProject {
            root_project: find_package()?,
            targets: Vec::new(),
        };
        let fuzz_project = project.path();
        let root_project_name = project.root_project_name()?;
        let target: String = args.value_of_os("target").expect("target shoud have a default value").to_os_string()
            .into_string().map_err(|_| "target must be valid unicode")?;

        // TODO: check if the project is already initialized
        fs::create_dir(&fuzz_project)?;
        fs::create_dir(fuzz_project.join(FUZZ_TARGETS_DIR))?;

        let mut cargo = fs::File::create(fuzz_project.join("Cargo.toml"))?;
        cargo.write_fmt(toml_template!(root_project_name))?;

        let mut ignore = fs::File::create(fuzz_project.join(".gitignore"))?;
        ignore.write_fmt(gitignore_template!())?;

        project.create_target_template(&target)
               .chain_err(|| format!("could not create template file for target {:?}", target))?;
        Ok(project)
    }

    fn list_targets(&self) -> Result<()> {
        for bin in &self.targets {
            utils::print_message(bin, term::color::GREEN);
        }
        Ok(())
    }

    fn add_target(&self, args: &ArgMatches) -> Result<()> {
        let target = get_target(args)?;
        // Create corpus and artifact directories for the newly added target
        self.corpus_for(&target)?;
        self.artifacts_for(&target)?;
        self.create_target_template(&target)
            .chain_err(|| format!("could not add target {:?}", target))
    }

    /// Add a new fuzz target script with a given name
    fn create_target_template(&self, target: &str) -> Result<()> {
        let target_path = self.target_path(target);
        let mut script = fs::OpenOptions::new().write(true).create_new(true).open(&target_path)
            .chain_err(|| format!("could not create target script file at {:?}", target_path))?;
        script.write_fmt(target_template!())?;

        let mut cargo = fs::OpenOptions::new().append(true)
            .open(self.manifest_path())?;
        Ok(cargo.write_fmt(toml_bin_template!(target))?)
    }

    fn cargo(&self, name: &str, args: &ArgMatches) -> Result<process::Command> {
        let sanitizer: &str = args.value_of("sanitizer").expect("no sanitizer");
        let target = get_target(args)?;
        let target_triple = args.value_of_os("TRIPLE").expect("no triple");

        let mut cmd = process::Command::new("cargo");
        cmd.arg(name)
            .arg("--manifest-path").arg(self.manifest_path())
            .arg("--verbose")
            .arg("--bin").arg(target)
            // --target=<TARGET> won't pass rustflags to build scripts
            .arg("--target").arg(target_triple);
        if args.is_present("release") {
            cmd.arg("--release");
        }
        if args.is_present("no_default_features") {
            cmd.arg("--no-default-features");
        }
        if args.is_present("all_features") {
            cmd.arg("--all-features");
        }
        if let Some(value) = args.value_of("features") {
            cmd.arg("--features").arg(value);
        }

        let mut rustflags: String = format!(
            "--cfg fuzzing \
             -Cpasses=sancov \
             -Cllvm-args=-sanitizer-coverage-level=4 \
             -Cllvm-args=-sanitizer-coverage-trace-compares \
             -Cllvm-args=-sanitizer-coverage-inline-8bit-counters \
             -Cllvm-args=-sanitizer-coverage-trace-geps \
             -Cllvm-args=-sanitizer-coverage-prune-blocks=0 \
             -Cllvm-args=-sanitizer-coverage-pc-table \
             -Zsanitizer={sanitizer}",
            sanitizer = sanitizer,
        );
        if target_triple.to_str().expect("target triple not utf-8").contains("-linux-") {
            rustflags.push_str(" -Cllvm-args=-sanitizer-coverage-stack-depth");
        }
        if args.is_present("debug_assertions") {
            rustflags.push_str(" -Cdebug-assertions");
        }

        let other_flags = env::var("RUSTFLAGS").unwrap_or_default();
        if !other_flags.is_empty() {
            rustflags.push_str(" ");
            rustflags.push_str(&other_flags);
        }
        cmd.env("RUSTFLAGS", rustflags);


        // For asan and tsan we have default options. Merge them to the given
        // options, so users can still provide their own options to e.g. disable
        // the leak sanitizer.  Options are colon-separated.
        match sanitizer {
            "address" => {
                let mut asan_opts = env::var("ASAN_OPTIONS").unwrap_or_default();
                if !asan_opts.is_empty() {
                    asan_opts.push(':');
                }
                asan_opts.push_str("detect_odr_violation=0");
                cmd.env("ASAN_OPTIONS", asan_opts);
            }

            "thread" => {
                let mut tsan_opts = env::var("TSAN_OPTIONS").unwrap_or_default();
                if !tsan_opts.is_empty() {
                    tsan_opts.push(':');
                }
                tsan_opts.push_str("report_signal_unsafe=0");
                cmd.env("TSAN_OPTIONS", tsan_opts);
            }

            _ => {}
        }

        Ok(cmd)
    }

    fn cmd(&self, args: &ArgMatches) -> Result<process::Command> {
        let target = get_target(args)?;
        let mut cmd = self.cargo("run", args)?;
        let mut artifact_arg = ffi::OsString::from("-artifact_prefix=");
        artifact_arg.push(self.artifacts_for(&target)?);

        cmd.arg("--")
           .arg(artifact_arg);

        Ok(cmd)
    }

    /// Fuzz a given fuzz target
    fn exec_fuzz<'a>(&self, args: &ArgMatches<'a>) -> Result<()> {
        let target = get_target(args)?;

        let mut cmd = self.cargo("build", args)?;
        let status = cmd.status()
            .chain_err(|| format!("could not execute: {:?}", cmd))?;
        if !status.success() {
            return Err(format!("could not build fuzz script: {:?}", cmd).into());
        }

        let mut cmd = self.cmd(args)?;

        if let Some(args) = args.values_of_os("ARGS") {
            for arg in args {
                cmd.arg(arg);
            }
        }
        if let Some(corpus) = args.values_of_os("CORPUS") {
            for arg in corpus {
                cmd.arg(arg);
            }
        } else {
            cmd.arg(self.corpus_for(&target)?);
        }

        let jobs: u16 = args.value_of("JOBS").expect("no jobs")
            .parse().expect("validation");
        if jobs != 1 {
            cmd.arg(format!("-fork={}", jobs));
        }
        exec_cmd(&mut cmd).chain_err(|| format!("could not execute command: {:?}", cmd))?;
        Ok(())
    }

    fn exec_tmin(&self, args: &ArgMatches) -> Result<()> {
        let mut cmd = self.cmd(args)?;

        let runs: u32 = args.value_of("runs").unwrap()
            .parse().expect("runs should be int");

        cmd.arg("-minimize_crash=1")
           .arg(format!("-runs={}", runs))
           .arg(args.value_of("CRASH").unwrap());
        exec_cmd(&mut cmd)
            .chain_err(|| format!("could not execute command: {:?}", cmd))?;
        Ok(())
    }

    fn exec_cmin(&self, args: &ArgMatches) -> Result<()> {
        let mut cmd = self.cmd(args)?;

        let corpus = if let Some(corpus) = args.value_of("CORPUS") {
            corpus.to_owned()
        } else {
            self.corpus_for(&get_target(args)?)?
                .to_str().expect("CORPUS should be valid unicode")
                .to_owned()
        };

        let tmp = tempdir::TempDir::new_in(self.path(), "cmin")?;

        fs::create_dir(tmp.path().join("corpus"))?;

        cmd.arg("-merge=1")
            .arg(tmp.path().join("corpus"))
            .arg(&corpus);

        // Spawn cmd in child process instead of exec-ing it
        let status = cmd.status()
            .chain_err(|| format!("could not execute command: {:?}", cmd))?;
        if status.success() {
            // move corpus directory into tmp to auto delete it
            fs::rename(&corpus, tmp.path().join("old"))?;
            fs::rename(tmp.path().join("corpus"), corpus)?;
        } else {
            println!("Failed to minimize corpus: {}", status);
        }

        Ok(())
    }

    fn path(&self) -> path::PathBuf {
        self.root_project.join("fuzz")
    }

    fn manifest_path(&self) -> path::PathBuf {
        self.path().join("Cargo.toml")
    }

    fn corpus_for(&self, target: &str) -> Result<path::PathBuf> {
        let mut p = self.path();
        p.push("corpus");
        p.push(target);
        fs::create_dir_all(&p)
            .chain_err(|| format!("could not make a corpus directory at {:?}", p))?;
        Ok(p)
    }

    fn artifacts_for(&self, target: &str) -> Result<path::PathBuf> {
        let mut p = self.path();
        p.push("artifacts");
        p.push(target);
        p.push(""); // trailing slash, necessary for libfuzzer, because it does simple concat
        fs::create_dir_all(&p)
            .chain_err(|| format!("could not make a artifact directory at {:?}", p))?;
        Ok(p)
    }

    fn target_path(&self, target: &str) -> path::PathBuf {
        let mut root = self.path();
        if root.join(FUZZ_TARGETS_DIR_OLD).exists() {
            println!("warning: The `fuzz/fuzzers/` directory has renamed to `fuzz/fuzz_targets/`. \
                      Please rename the directory as such. This will become a hard error in the \
                      future.");
            root.push(FUZZ_TARGETS_DIR_OLD);
        } else {
            root.push(FUZZ_TARGETS_DIR);
        }
        root.push(target);
        root.set_extension("rs");
        root
    }

    fn manifest(&self) -> Result<toml::Value> {
        let filename = self.manifest_path();
        let mut file = fs::File::open(&filename).chain_err(||
            format!("could not read the manifest file: {:?}", filename)
        )?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        toml::from_slice(&data).chain_err(||
            format!("could not decode the manifest file at {:?}", filename)
        )
    }

    fn root_project_name(&self) -> Result<String> {
        let filename = self.root_project.join("Cargo.toml");
        let mut file = fs::File::open(&filename)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let value: toml::Value = toml::from_slice(&data)?;
        let name = value.as_table().and_then(|v| v.get("package"))
                                   .and_then(toml::Value::as_table)
                                   .and_then(|v| v.get("name"))
                                   .and_then(toml::Value::as_str);
        if let Some(name) = name {
            Ok(String::from(name))
        } else {
            Err(format!("{:?} (package.name) is malformed", filename).into())
        }
    }
}

fn collect_targets(value: &toml::Value) -> Vec<String> {
    let bins = value.as_table().and_then(|v| v.get("bin"))
                               .and_then(toml::Value::as_array);
    if let Some(bins) = bins {
        bins.iter().map(|bin|
            bin.as_table().and_then(|v| v.get("name")).and_then(toml::Value::as_str)
        ).filter_map(|name| name.map(String::from)).collect()
    } else {
        Vec::new()
    }
}

fn is_fuzz_manifest(value: &toml::Value) -> bool {
    let is_fuzz = value.as_table().and_then(|v| v.get("package"))
                                  .and_then(toml::Value::as_table)
                                  .and_then(|v| v.get("metadata"))
                                  .and_then(toml::Value::as_table)
                                  .and_then(|v| v.get("cargo-fuzz"))
                                  .and_then(toml::Value::as_bool);
    is_fuzz == Some(true)
}

/// Returns the path for the first found non-fuzz Cargo package
fn find_package() -> Result<path::PathBuf> {
    let mut dir = env::current_dir()?;
    let mut data = Vec::new();
    loop {
        let manifest_path = dir.join("Cargo.toml");
        match fs::File::open(&manifest_path) {
            Err(_) => {},
            Ok(mut f) => {
                data.clear();
                f.read_to_end(&mut data)?;
                let value: toml::Value = toml::from_slice(&data)
                    .chain_err(||
                        format!("could not decode the manifest file at {:?}", manifest_path)
                    )?;
                if !is_fuzz_manifest(&value) {
                    // Not a cargo-fuzz project => must be a proper cargo project :)
                    return Ok(dir);
                }
            }
        }
        if !dir.pop() { break; }
    }
    Err("could not find a cargo project".into())
}

#[cfg(unix)]
fn exec_cmd(cmd: &mut process::Command) -> Result<process::ExitStatus> {
    use std::os::unix::process::CommandExt;
    Err(cmd.exec().into())
}

#[cfg(not(unix))]
fn exec_cmd(cmd: &mut process::Command) -> Result<process::ExitStatus> {
    cmd.status().map_err(|e| e.into())
}
