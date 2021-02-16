mod add;
mod build;
mod cmin;
mod fmt;
mod init;
mod list;
mod run;
mod tmin;

pub use self::{
    add::Add, build::Build, cmin::Cmin, fmt::Fmt, init::Init, list::List, run::Run, tmin::Tmin,
};

use std::fmt as stdfmt;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(Debug, Clone, Copy, PartialEq)]
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

impl FromStr for Sanitizer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "address" => Ok(Sanitizer::Address),
            "leak" => Ok(Sanitizer::Leak),
            "memory" => Ok(Sanitizer::Memory),
            "thread" => Ok(Sanitizer::Thread),
            "none" => Ok(Sanitizer::None),
            _ => Err(format!("unknown sanitizer: {}", s)),
        }
    }
}

#[derive(Clone, Debug, StructOpt, PartialEq)]
pub struct BuildOptions {
    #[structopt(short = "D", long = "dev", conflicts_with = "release")]
    /// Build artifacts in development mode, without optimizations
    pub dev: bool,

    #[structopt(short = "O", long = "release", conflicts_with = "dev")]
    /// Build artifacts in release mode, with optimizations
    pub release: bool,

    #[structopt(short = "a", long = "debug-assertions")]
    /// Build artifacts with debug assertions and overflow checks enabled (default if not -O)
    pub debug_assertions: bool,

    /// Build target with verbose output from `cargo build`
    #[structopt(short = "v", long = "verbose")]
    pub verbose: bool,

    #[structopt(long = "no-default-features")]
    /// Build artifacts with default Cargo features disabled
    pub no_default_features: bool,

    #[structopt(
        long = "all-features",
        conflicts_with = "no-default-features",
        conflicts_with = "features"
    )]
    /// Build artifacts with all Cargo features enabled
    pub all_features: bool,

    #[structopt(long = "features")]
    /// Build artifacts with given Cargo feature enabled
    pub features: Option<String>,

    #[structopt(
        short = "s",
        long = "sanitizer",
        possible_values(&["address", "leak", "memory", "thread", "none"]),
        default_value = "address",
    )]
    /// Use a specific sanitizer
    pub sanitizer: Sanitizer,

    #[structopt(
        name = "triple",
        long = "target",
        default_value(crate::utils::default_target())
    )]
    /// Target triple of the fuzz target
    pub triple: String,

    #[structopt(short = "Z", value_name = "FLAG")]
    /// Unstable (nightly-only) flags to Cargo
    pub unstable_flags: Vec<String>,

    #[structopt(long = "target-dir")]
    /// Target dir option to pass to cargo build.
    pub target_dir: Option<String>,

    #[structopt(long = "coverage")]
    /// Instrument program code with source-based code coverage information.
    /// The profiler data will be saved in the given file.
    pub coverage_output_file: Option<String>,
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

        if let Some(filename) = &self.coverage_output_file {
            write!(f, "  --coverage={}", filename)?;
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
            sanitizer: Sanitizer::Address,
            triple: String::from(crate::utils::default_target()),
            unstable_flags: Vec::new(),
            target_dir: None,
            coverage_output_file: None,
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
                coverage_output_file: None,
                ..default_opts
            },
        ];

        for case in opts {
            assert_eq!(
                case,
                BuildOptions::from_clap(
                    &BuildOptions::clap().get_matches_from(case.to_string().split(' '))
                )
            );
        }
    }
}
