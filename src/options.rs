mod add;
mod build;
mod check;
mod cmin;
mod coverage;
mod fmt;
mod init;
mod list;
mod run;
mod tmin;

pub use self::{
    add::Add, build::Build, check::Check, cmin::Cmin, coverage::Coverage, fmt::Fmt, init::Init,
    list::List, run::Run, tmin::Tmin,
};

use clap::{Parser, ValueEnum};
use std::{fmt as stdfmt, path::PathBuf};

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum Sanitizer {
    Address,
    Leak,
    Memory,
    Thread,
    None,
}

impl stdfmt::Display for Sanitizer {
    fn fmt(&self, f: &mut stdfmt::Formatter) -> stdfmt::Result {
        write!(
            f,
            "{}",
            match self {
                Sanitizer::Address => "address",
                Sanitizer::Leak => "leak",
                Sanitizer::Memory => "memory",
                Sanitizer::Thread => "thread",
                Sanitizer::None => "",
            }
        )
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BuildMode {
    Build,
    Check,
}

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
pub struct BuildOptions {
    #[arg(short = 'D', long, conflicts_with = "release")]
    /// Build artifacts in development mode, without optimizations
    pub dev: bool,

    #[arg(short = 'O', long, conflicts_with = "dev")]
    /// Build artifacts in release mode, with optimizations
    pub release: bool,

    #[arg(short = 'a', long)]
    /// Build artifacts with debug assertions and overflow checks enabled (default if not -O)
    pub debug_assertions: bool,

    /// Build target with verbose output from `cargo build`
    #[arg(short = 'v', long)]
    pub verbose: bool,

    #[arg(long)]
    /// Build artifacts with default Cargo features disabled
    pub no_default_features: bool,

    #[arg(long, conflicts_with_all = &["no_default_features", "features"])]
    /// Build artifacts with all Cargo features enabled
    pub all_features: bool,

    #[arg(long)]
    /// Build artifacts with given Cargo feature enabled
    pub features: Option<String>,

    #[arg(short, long, value_enum, default_value = "address")]
    /// Use a specific sanitizer
    pub sanitizer: Sanitizer,

    #[arg(long = "build-std")]
    /// Pass -Zbuild-std to Cargo, which will build the standard library with all the build
    /// settings for the fuzz target, including debug assertions, and a sanitizer if requested.
    /// Currently this conflicts with coverage instrumentation but -Zbuild-std enables detecting
    /// more bugs so this option defaults to true, but when using `cargo fuzz coverage` it
    /// defaults to false.
    pub build_std: bool,

    #[arg(short, long = "careful")]
    /// enable "careful" mode: inspired by https://github.com/RalfJung/cargo-careful, this enables
    /// building the fuzzing harness along with the standard library (implies --build-std) with
    /// debug assertions and extra const UB and init checks.
    pub careful_mode: bool,

    #[arg(long = "target", default_value(crate::utils::default_target()))]
    /// Target triple of the fuzz target
    pub triple: String,

    #[arg(short = 'Z', value_name = "FLAG")]
    /// Unstable (nightly-only) flags to Cargo
    pub unstable_flags: Vec<String>,

    #[arg(long)]
    /// Target dir option to pass to cargo build.
    pub target_dir: Option<String>,

    #[arg(skip = false)]
    /// Instrument program code with source-based code coverage information.
    /// This build option will be automatically used when running `cargo fuzz coverage`.
    /// The option will not be shown to the user, which is ensured by the `skip` attribute.
    /// The attribute takes a default value `false`, ensuring that by default,
    /// the coverage option will be disabled).
    pub coverage: bool,

    /// Dead code is linked by default to prevent a potential error with some
    /// optimized targets. This flag allows you to opt out of it.
    #[arg(long)]
    pub strip_dead_code: bool,

    /// By default the 'cfg(fuzzing)' compilation configuration is set. This flag
    /// allows you to opt out of it.
    #[arg(long)]
    pub no_cfg_fuzzing: bool,

    #[arg(long)]
    /// Don't build with the `sanitizer-coverage-trace-compares` LLVM argument
    ///
    ///  Using this may improve fuzzer throughput at the cost of worse coverage accuracy.
    /// It also allows older CPUs lacking the `popcnt` instruction to use `cargo-fuzz`;
    /// the `*-trace-compares` instrumentation assumes that the instruction is
    /// available.
    pub no_trace_compares: bool,
}

impl stdfmt::Display for BuildOptions {
    fn fmt(&self, f: &mut stdfmt::Formatter) -> stdfmt::Result {
        if self.dev {
            write!(f, " -D")?;
        }

        if self.release {
            write!(f, " -O")?;
        }

        if self.debug_assertions {
            write!(f, " -a")?;
        }

        if self.verbose {
            write!(f, " -v")?;
        }

        if self.no_default_features {
            write!(f, " --no-default-features")?;
        }

        if self.all_features {
            write!(f, " --all-features")?;
        }

        if let Some(feature) = &self.features {
            write!(f, " --features={}", feature)?;
        }

        match self.sanitizer {
            Sanitizer::None => write!(f, " --sanitizer=none")?,
            Sanitizer::Address => {}
            _ => write!(f, " --sanitizer={}", self.sanitizer)?,
        }

        if self.triple != crate::utils::default_target() {
            write!(f, " --target={}", self.triple)?;
        }

        for flag in &self.unstable_flags {
            write!(f, " -Z{}", flag)?;
        }

        if let Some(target_dir) = &self.target_dir {
            write!(f, " --target-dir={}", target_dir)?;
        }

        if self.coverage {
            write!(f, " --coverage")?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
pub struct FuzzDirWrapper {
    /// The path to the fuzz project directory.
    #[arg(long)]
    pub fuzz_dir: Option<PathBuf>,
}

impl stdfmt::Display for FuzzDirWrapper {
    fn fmt(&self, f: &mut stdfmt::Formatter) -> stdfmt::Result {
        if let Some(ref elem) = self.fuzz_dir {
            write!(f, " --fuzz-dir={}", elem.display())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn display_build_options() {
        let default_opts = BuildOptions {
            dev: false,
            release: false,
            debug_assertions: false,
            verbose: false,
            no_default_features: false,
            all_features: false,
            features: None,
            build_std: false,
            careful_mode: false,
            sanitizer: Sanitizer::Address,
            triple: String::from(crate::utils::default_target()),
            unstable_flags: Vec::new(),
            target_dir: None,
            coverage: false,
            strip_dead_code: false,
            no_cfg_fuzzing: false,
            no_trace_compares: false,
        };

        let opts = vec![
            default_opts.clone(),
            BuildOptions {
                dev: true,
                ..default_opts.clone()
            },
            BuildOptions {
                release: true,
                ..default_opts.clone()
            },
            BuildOptions {
                debug_assertions: true,
                ..default_opts.clone()
            },
            BuildOptions {
                verbose: true,
                ..default_opts.clone()
            },
            BuildOptions {
                no_default_features: true,
                ..default_opts.clone()
            },
            BuildOptions {
                all_features: true,
                ..default_opts.clone()
            },
            BuildOptions {
                features: Some(String::from("features")),
                ..default_opts.clone()
            },
            BuildOptions {
                sanitizer: Sanitizer::None,
                ..default_opts.clone()
            },
            BuildOptions {
                triple: String::from("custom_triple"),
                ..default_opts.clone()
            },
            BuildOptions {
                unstable_flags: vec![String::from("unstable"), String::from("flags")],
                ..default_opts.clone()
            },
            BuildOptions {
                target_dir: Some(String::from("/tmp/test")),
                ..default_opts.clone()
            },
            BuildOptions {
                coverage: false,
                ..default_opts
            },
        ];

        for case in opts {
            assert_eq!(case, BuildOptions::parse_from(case.to_string().split(' ')));
        }
    }
}
