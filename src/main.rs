// Copyright 2016 rust-fuzz developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use anyhow::{anyhow, bail, Context, Result};
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::{env, ffi, fmt, fs, process};
use structopt::StructOpt;

#[macro_use]
mod templates;
mod utils;

static FUZZ_TARGETS_DIR_OLD: &'static str = "fuzzers";
static FUZZ_TARGETS_DIR: &'static str = "fuzz_targets";

// It turns out that `clap`'s `long_about()` makes `cargo fuzz --help`
// unreadable, and its `before_help()` injects our long about text before the
// version, so change the default template slightly.
const LONG_ABOUT_TEMPLATE: &'static str = "\
{bin} {version}
{about}

USAGE:
    {usage}

{before-help}

{all-args}

{after-help}";

const RUN_BEFORE_HELP: &'static str = "\
The fuzz target name is the same as the name of the fuzz target script in
fuzz/fuzz_targets/, i.e. the name picked when running `cargo fuzz add`.

This will run the script inside the fuzz target with varying inputs until it
finds a crash, at which point it will save the crash input to the artifact
directory, print some output, and exit. Unless you configure it otherwise (see
libFuzzer options below), this will run indefinitely.";

const RUN_AFTER_HELP: &'static str = "\

A full list of libFuzzer options can be found at
http://llvm.org/docs/LibFuzzer.html#options

You can also get this by running `cargo fuzz run fuzz_target -- -help=1`

Some useful options (to be used as `cargo fuzz run fuzz_target -- <options>`)
include:

  * `-max_len=<len>`: Will limit the length of the input string to `<len>`

  * `-runs=<number>`: Will limit the number of tries (runs) before it gives up

  * `-max_total_time=<time>`: Will limit the amount of time to fuzz before it
    gives up

  * `-timeout=<time>`: Will limit the amount of time for a single run before it
    considers that run a failure

  * `-only_ascii`: Only provide ASCII input

  * `-dict=<file>`: Use a keyword dictionary from specified file. See
    http://llvm.org/docs/LibFuzzer.html#dictionaries\
";

/// A trait for running our various commands.
trait RunCommand {
    /// Run this command!
    fn run_command(&mut self) -> Result<()>;
}

#[derive(Clone, Debug, StructOpt)]
#[structopt(
    setting(structopt::clap::AppSettings::SubcommandRequiredElseHelp),
    setting(structopt::clap::AppSettings::GlobalVersion),
    version(option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0")),
    about(option_env!("CARGO_PKG_DESCRIPTION").unwrap_or("")),
    // Cargo passes in the subcommand name to the invoked executable. Use a
    // hidden, optional positional argument to deal with it.
    arg(structopt::clap::Arg::with_name("dummy")
        .possible_value("fuzz")
        .required(false)
        .hidden(true)),

)]
enum Command {
    /// Initialize the fuzz directory
    Init(Init),

    /// Add a new fuzz target
    Add(Add),

    /// List all the existing fuzz targets
    List(List),

    #[structopt(
        template(LONG_ABOUT_TEMPLATE),
        before_help(RUN_BEFORE_HELP),
        after_help(RUN_AFTER_HELP)
    )]
    /// Run a fuzz target
    Run(Run),

    /// Minify a corpus
    Cmin(Cmin),

    /// Minify a test case
    Tmin(Tmin),
}

impl RunCommand for Command {
    fn run_command(&mut self) -> Result<()> {
        match self {
            Command::Init(x) => x.run_command(),
            Command::Add(x) => x.run_command(),
            Command::List(x) => x.run_command(),
            Command::Run(x) => x.run_command(),
            Command::Cmin(x) => x.run_command(),
            Command::Tmin(x) => x.run_command(),
        }
    }
}

#[derive(Clone, Debug, StructOpt)]
struct Init {
    #[structopt(
        short = "t",
        long = "target",
        required = false,
        default_value = "fuzz_target_1"
    )]
    /// Name of the first fuzz target to create
    target: String,
}

impl RunCommand for Init {
    fn run_command(&mut self) -> Result<()> {
        FuzzProject::init(self)?;
        Ok(())
    }
}

#[derive(Clone, Debug, StructOpt)]
struct Add {
    #[structopt(required = true)]
    /// Name of the new fuzz target
    target: String,
}

impl RunCommand for Add {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new()?;
        project.add_target(self)
    }
}

#[derive(Clone, Debug, StructOpt)]
struct List {}

impl RunCommand for List {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new()?;
        project.list_targets()
    }
}

#[derive(Debug, Clone, Copy)]
enum Sanitizer {
    Address,
    Leak,
    Memory,
    Thread,
}

impl fmt::Display for Sanitizer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Sanitizer::Address => "address",
                Sanitizer::Leak => "leak",
                Sanitizer::Memory => "memory",
                Sanitizer::Thread => "thread",
            }
        )
    }
}

impl FromStr for Sanitizer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "address" => Ok(Sanitizer::Address),
            "leak" => Ok(Sanitizer::Leak),
            "memory" => Ok(Sanitizer::Memory),
            "thread" => Ok(Sanitizer::Thread),
            _ => Err(format!("unknown sanitizer: {}", s)),
        }
    }
}

#[derive(Clone, Debug, StructOpt)]
struct BuildOptions {
    #[structopt(short = "O", long = "release")]
    /// Build artifacts in release mode, with optimizations
    release: bool,

    #[structopt(short = "a", long = "debug-assertions")]
    /// Build artifacts with debug assertions enabled (default if not -O)
    debug_assertions: bool,

    #[structopt(long = "no-default-features")]
    /// Build artifacts with default Cargo features disabled
    no_default_features: bool,

    #[structopt(
        long = "all-features",
        conflicts_with = "no-default-features",
        conflicts_with = "features"
    )]
    /// Build artifacts with all Cargo features enabled
    all_features: bool,

    #[structopt(long = "features")]
    /// Build artifacts with given Cargo feature enabled
    features: Option<String>,

    #[structopt(
        short = "s",
        long = "sanitizer",
        possible_values(&["address", "leak", "memory", "thread"]),
        default_value = "address",
    )]
    /// Use a specific sanitizer
    sanitizer: Sanitizer,

    #[structopt(
        name = "triple",
        long = "target",
        default_value(utils::default_target())
    )]
    /// Target triple of the fuzz target
    triple: String,

    #[structopt(required(true))]
    /// Name of the fuzz target
    target: String,
}

#[derive(Clone, Debug, StructOpt)]
struct Run {
    #[structopt(flatten)]
    build: BuildOptions,

    /// Custom corpus directories or artifact files.
    corpus: Vec<String>,

    #[structopt(
        short = "j",
        long = "jobs",
        default_value = "1",
        validator(|v| Err(From::from(match v.parse::<u16>() {
            Ok(0) => "0 jobs?",
            Err(_) => "must be a valid integer representing a sane number of jobs",
            _ => return Ok(()),
        }))),
    )]
    /// Number of concurrent jobs to run
    jobs: u32,

    #[structopt(last(true))]
    /// Additional libFuzzer arguments passed through to the binary
    args: Vec<String>,
}

impl RunCommand for Run {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new()?;
        project.exec_fuzz(self)
    }
}

#[derive(Clone, Debug, StructOpt)]
struct Cmin {
    #[structopt(flatten)]
    build: BuildOptions,

    #[structopt(parse(from_os_str))]
    /// The corpus directory to minify into
    corpus: Option<PathBuf>,
}

impl RunCommand for Cmin {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new()?;
        project.exec_cmin(self)
    }
}

#[derive(Clone, Debug, StructOpt)]
struct Tmin {
    #[structopt(flatten)]
    build: BuildOptions,

    #[structopt(
        short = "r",
        long = "runs",
        default_value = "255",
        validator(|v| Err(From::from(match v.parse::<u32>() {
            Ok(0) => "0 jobs?",
            Err(_) => "must be a valid integer representing a sane number of jobs",
            _ => return Ok(()),
        }))),
    )]
    /// Number of minimization attempts to perform
    runs: u32,

    #[structopt(parse(from_os_str))]
    /// Path to the failing test case to be minimized
    test_case: PathBuf,
}

impl RunCommand for Tmin {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new()?;
        project.exec_tmin(self)
    }
}

fn main() -> Result<()> {
    Command::from_args().run_command()
}

struct FuzzProject {
    /// Path to the root cargo project
    ///
    /// Not the project with fuzz targets, but the project being fuzzed
    root_project: PathBuf,
    targets: Vec<String>,
}

impl FuzzProject {
    fn new() -> Result<Self> {
        let mut project = FuzzProject {
            root_project: find_package()?,
            targets: Vec::new(),
        };
        let manifest = project.manifest()?;
        if !is_fuzz_manifest(&manifest) {
            bail!(
                "manifest `{}` does not look like a cargo-fuzz manifest. \
                 Add following lines to override:\n\
                 [package.metadata]\n\
                 cargo-fuzz = true",
                project.manifest_path().display()
            );
        }
        project.targets = collect_targets(&manifest);
        Ok(project)
    }

    /// Create the fuzz project structure
    ///
    /// This will not clone libfuzzer-sys
    fn init(init: &Init) -> Result<Self> {
        let project = FuzzProject {
            root_project: find_package()?,
            targets: Vec::new(),
        };
        let fuzz_project = project.path();
        let root_project_name = project.root_project_name()?;

        // TODO: check if the project is already initialized
        fs::create_dir(&fuzz_project)
            .with_context(|| format!("failed to create directory {}", fuzz_project.display()))?;

        let fuzz_targets_dir = fuzz_project.join(FUZZ_TARGETS_DIR);
        fs::create_dir(&fuzz_targets_dir).with_context(|| {
            format!("failed to create directory {}", fuzz_targets_dir.display())
        })?;

        let cargo_toml = fuzz_project.join("Cargo.toml");
        let mut cargo = fs::File::create(&cargo_toml)
            .with_context(|| format!("failed to create {}", cargo_toml.display()))?;
        cargo
            .write_fmt(toml_template!(root_project_name))
            .with_context(|| format!("failed to write to {}", cargo_toml.display()))?;

        let gitignore = fuzz_project.join(".gitignore");
        let mut ignore = fs::File::create(&gitignore)
            .with_context(|| format!("failed to create {}", gitignore.display()))?;
        ignore
            .write_fmt(gitignore_template!())
            .with_context(|| format!("failed to write to {}", gitignore.display()))?;

        project
            .create_target_template(&init.target)
            .with_context(|| {
                format!(
                    "could not create template file for target {:?}",
                    init.target
                )
            })?;
        Ok(project)
    }

    fn list_targets(&self) -> Result<()> {
        for bin in &self.targets {
            println!("{}", bin);
        }
        Ok(())
    }

    fn add_target(&self, add: &Add) -> Result<()> {
        // Create corpus and artifact directories for the newly added target
        self.corpus_for(&add.target)?;
        self.artifacts_for(&add.target)?;
        self.create_target_template(&add.target)
            .with_context(|| format!("could not add target {:?}", add.target))
    }

    /// Add a new fuzz target script with a given name
    fn create_target_template(&self, target: &str) -> Result<()> {
        let target_path = self.target_path(target);
        let mut script = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target_path)
            .with_context(|| format!("could not create target script file at {:?}", target_path))?;
        script.write_fmt(target_template!())?;

        let mut cargo = fs::OpenOptions::new()
            .append(true)
            .open(self.manifest_path())?;
        Ok(cargo.write_fmt(toml_bin_template!(target))?)
    }

    fn cargo(&self, name: &str, build: &BuildOptions) -> Result<process::Command> {
        let mut cmd = process::Command::new("cargo");
        cmd.arg(name)
            .arg("--manifest-path")
            .arg(self.manifest_path())
            .arg("--verbose")
            .arg("--bin")
            .arg(&build.target)
            // --target=<TARGET> won't pass rustflags to build scripts
            .arg("--target")
            .arg(&build.triple);
        if build.release {
            cmd.arg("--release");
        }
        if build.no_default_features {
            cmd.arg("--no-default-features");
        }
        if build.all_features {
            cmd.arg("--all-features");
        }
        if let Some(ref features) = build.features {
            cmd.arg("--features").arg(features);
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
             -Clink-dead-code \
             -Zsanitizer={sanitizer}",
            sanitizer = build.sanitizer,
        );
        if build.triple.contains("-linux-") {
            rustflags.push_str(" -Cllvm-args=-sanitizer-coverage-stack-depth");
        }
        if build.debug_assertions {
            rustflags.push_str(" -Cdebug-assertions");
        }

        if let Ok(other_flags) = env::var("RUSTFLAGS") {
            rustflags.push_str(" ");
            rustflags.push_str(&other_flags);
        }
        cmd.env("RUSTFLAGS", rustflags);

        // For asan and tsan we have default options. Merge them to the given
        // options, so users can still provide their own options to e.g. disable
        // the leak sanitizer.  Options are colon-separated.
        match build.sanitizer {
            Sanitizer::Address => {
                let mut asan_opts = env::var("ASAN_OPTIONS").unwrap_or_default();
                if !asan_opts.is_empty() {
                    asan_opts.push(':');
                }
                asan_opts.push_str("detect_odr_violation=0");
                cmd.env("ASAN_OPTIONS", asan_opts);
            }

            Sanitizer::Thread => {
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

    fn cmd(&self, build: &BuildOptions) -> Result<process::Command> {
        let mut cmd = self.cargo("run", build)?;

        let mut artifact_arg = ffi::OsString::from("-artifact_prefix=");
        artifact_arg.push(self.artifacts_for(&build.target)?);
        cmd.arg("--").arg(artifact_arg);

        Ok(cmd)
    }

    /// Fuzz a given fuzz target
    fn exec_fuzz<'a>(&self, run: &Run) -> Result<()> {
        let mut cmd = self.cargo("build", &run.build)?;
        let status = cmd
            .status()
            .with_context(|| format!("could not execute: {:?}", cmd))?;
        if !status.success() {
            bail!("could not build fuzz script: {:?}", cmd);
        }

        let mut cmd = self.cmd(&run.build)?;

        for arg in &run.args {
            cmd.arg(arg);
        }
        if !run.corpus.is_empty() {
            for corpus in &run.corpus {
                cmd.arg(corpus);
            }
        } else {
            cmd.arg(self.corpus_for(&run.build.target)?);
        }

        if run.jobs != 1 {
            cmd.arg(format!("-fork={}", run.jobs));
        }
        exec_cmd(&mut cmd).with_context(|| format!("could not execute command: {:?}", cmd))?;
        Ok(())
    }

    fn exec_tmin(&self, tmin: &Tmin) -> Result<()> {
        let mut cmd = self.cmd(&tmin.build)?;

        cmd.arg("-minimize_crash=1")
            .arg(format!("-runs={}", tmin.runs))
            .arg(&tmin.test_case);
        exec_cmd(&mut cmd).with_context(|| format!("could not execute command: {:?}", cmd))?;
        Ok(())
    }

    fn exec_cmin(&self, cmin: &Cmin) -> Result<()> {
        let mut cmd = self.cmd(&cmin.build)?;

        let corpus = if let Some(corpus) = cmin.corpus.clone() {
            corpus
        } else {
            self.corpus_for(&cmin.build.target)?
        };
        let corpus = corpus
            .to_str()
            .ok_or_else(|| anyhow!("corpus must be valid unicode"))?
            .to_owned();

        let tmp = tempdir::TempDir::new_in(self.path(), "cmin")?;
        let tmp_corpus = tmp.path().join("corpus");
        fs::create_dir(&tmp_corpus)?;

        cmd.arg("-merge=1").arg(&tmp_corpus).arg(&corpus);

        // Spawn cmd in child process instead of exec-ing it
        let status = cmd
            .status()
            .with_context(|| format!("could not execute command: {:?}", cmd))?;
        if status.success() {
            // move corpus directory into tmp to auto delete it
            fs::rename(&corpus, tmp.path().join("old"))?;
            fs::rename(tmp.path().join("corpus"), corpus)?;
        } else {
            println!("Failed to minimize corpus: {}", status);
        }

        Ok(())
    }

    fn path(&self) -> PathBuf {
        self.root_project.join("fuzz")
    }

    fn manifest_path(&self) -> PathBuf {
        self.path().join("Cargo.toml")
    }

    fn corpus_for(&self, target: &str) -> Result<PathBuf> {
        let mut p = self.path();
        p.push("corpus");
        p.push(target);
        fs::create_dir_all(&p)
            .with_context(|| format!("could not make a corpus directory at {:?}", p))?;
        Ok(p)
    }

    fn artifacts_for(&self, target: &str) -> Result<PathBuf> {
        let mut p = self.path();
        p.push("artifacts");
        p.push(target);

        // This adds a trailing slash, which is necessary for libFuzzer, because
        // it does simple string concatenation when joining paths.
        p.push("");

        fs::create_dir_all(&p)
            .with_context(|| format!("could not make a artifact directory at {:?}", p))?;

        Ok(p)
    }

    fn target_path(&self, target: &str) -> PathBuf {
        let mut root = self.path();
        if root.join(FUZZ_TARGETS_DIR_OLD).exists() {
            println!(
                "warning: The `fuzz/fuzzers/` directory has renamed to `fuzz/fuzz_targets/`. \
                 Please rename the directory as such. This will become a hard error in the \
                 future."
            );
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
        let mut file = fs::File::open(&filename)
            .with_context(|| format!("could not read the manifest file: {}", filename.display()))?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        toml::from_slice(&data).with_context(|| {
            format!(
                "could not decode the manifest file at {}",
                filename.display()
            )
        })
    }

    fn root_project_name(&self) -> Result<String> {
        let filename = self.root_project.join("Cargo.toml");
        let mut file = fs::File::open(&filename)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let value: toml::Value = toml::from_slice(&data)?;
        let name = value
            .as_table()
            .and_then(|v| v.get("package"))
            .and_then(toml::Value::as_table)
            .and_then(|v| v.get("name"))
            .and_then(toml::Value::as_str);
        if let Some(name) = name {
            Ok(String::from(name))
        } else {
            bail!("{} (package.name) is malformed", filename.display());
        }
    }
}

fn collect_targets(value: &toml::Value) -> Vec<String> {
    let bins = value
        .as_table()
        .and_then(|v| v.get("bin"))
        .and_then(toml::Value::as_array);
    if let Some(bins) = bins {
        bins.iter()
            .map(|bin| {
                bin.as_table()
                    .and_then(|v| v.get("name"))
                    .and_then(toml::Value::as_str)
            })
            .filter_map(|name| name.map(String::from))
            .collect()
    } else {
        Vec::new()
    }
}

fn is_fuzz_manifest(value: &toml::Value) -> bool {
    let is_fuzz = value
        .as_table()
        .and_then(|v| v.get("package"))
        .and_then(toml::Value::as_table)
        .and_then(|v| v.get("metadata"))
        .and_then(toml::Value::as_table)
        .and_then(|v| v.get("cargo-fuzz"))
        .and_then(toml::Value::as_bool);
    is_fuzz == Some(true)
}

/// Returns the path for the first found non-fuzz Cargo package
fn find_package() -> Result<PathBuf> {
    let mut dir = env::current_dir()?;
    let mut data = Vec::new();
    loop {
        let manifest_path = dir.join("Cargo.toml");
        match fs::File::open(&manifest_path) {
            Err(_) => {}
            Ok(mut f) => {
                data.clear();
                f.read_to_end(&mut data)
                    .with_context(|| format!("failed to read {}", manifest_path.display()))?;
                let value: toml::Value = toml::from_slice(&data).with_context(|| {
                    format!(
                        "could not decode the manifest file at {}",
                        manifest_path.display()
                    )
                })?;
                if !is_fuzz_manifest(&value) {
                    // Not a cargo-fuzz project => must be a proper cargo project :)
                    return Ok(dir);
                }
            }
        }
        if !dir.pop() {
            break;
        }
    }
    bail!("could not find a cargo project")
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
