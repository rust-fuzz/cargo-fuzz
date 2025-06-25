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
    /// Build artifacts in development mode, without optimizations
    #[arg(short = 'D', long, conflicts_with = "release")]
    pub dev: bool,

    /// Build artifacts in release mode, with optimizations
    #[arg(short = 'O', long, conflicts_with = "dev")]
    pub release: bool,

    /// Build artifacts with debug assertions and overflow checks enabled (default if not -O)
    #[arg(short = 'a', long)]
    pub debug_assertions: bool,

    /// Build target with verbose output from `cargo build`
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// Build artifacts with default Cargo features disabled
    #[arg(long)]
    pub no_default_features: bool,

    /// Build artifacts with all Cargo features enabled
    #[arg(long, conflicts_with_all = &["no_default_features", "features"])]
    pub all_features: bool,

    /// Build artifacts with given Cargo feature enabled
    #[arg(long)]
    pub features: Option<String>,

    /// Use a specific sanitizer
    #[arg(short, long, value_enum, default_value = "address")]
    pub sanitizer: Sanitizer,

    /// Pass -Zbuild-std to Cargo, which will build the standard library with all the build
    /// settings for the fuzz target, including debug assertions, and a sanitizer if requested.
    /// Currently this conflicts with coverage instrumentation but -Zbuild-std enables detecting
    /// more bugs so this option defaults to true, but when using `cargo fuzz coverage` it
    /// defaults to false.
    #[arg(long = "build-std")]
    pub build_std: bool,

    /// enable "careful" mode: inspired by https://github.com/RalfJung/cargo-careful, this enables
    /// building the fuzzing harness along with the standard library (implies --build-std) with
    /// debug assertions and extra const UB and init checks.
    #[arg(short, long = "careful")]
    pub careful_mode: bool,

    /// Target triple of the fuzz target
    #[arg(long = "target", default_value(crate::utils::default_target()))]
    pub triple: String,

    /// Unstable (nightly-only) flags to Cargo
    #[arg(short = 'Z', value_name = "FLAG")]
    pub unstable_flags: Vec<String>,

    /// Target dir option to pass to cargo build.
    #[arg(long)]
    pub target_dir: Option<String>,

    /// Instrument program code with source-based code coverage information.
    /// This build option will be automatically used when running `cargo fuzz coverage`.
    /// The option will not be shown to the user, which is ensured by the `skip` attribute.
    /// The attribute takes a default value `false`, ensuring that by default,
    /// the coverage option will be disabled).
    #[arg(skip = false)]
    pub coverage: bool,

    /// Number of codegen units to use. Default is 1 in non-dev builds. 16 may
    /// be a good choice if you want faster fuzz builds at the cost of somewhat
    /// slower fuzz runs.
    #[arg(long)]
    pub codegen_units: Option<usize>,

    /// Dead code is stripped by default.
    /// This flag allows you to opt out and always include dead code.
    /// Please note, this could trigger unexpected behavior or even ICEs in the compiler.
    //
    // `Option<Option<bool>>` gives an optional argument with an optional value.
    #[arg(long)]
    pub strip_dead_code: Option<Option<bool>>,

    /// By default the 'cfg(fuzzing)' compilation configuration is set. This flag
    /// allows you to opt out of it.
    #[arg(long)]
    pub no_cfg_fuzzing: bool,

    /// Don't build with the `sanitizer-coverage-trace-compares` LLVM argument
    ///
    ///  Using this may improve fuzzer throughput at the cost of worse coverage accuracy.
    /// It also allows older CPUs lacking the `popcnt` instruction to use `cargo-fuzz`;
    /// the `*-trace-compares` instrumentation assumes that the instruction is
    /// available.
    #[arg(long)]
    pub no_trace_compares: bool,

    /// Enables `sanitizer-coverage-trace-divs` LLVM instrumentation
    ///
    /// When set to `true`, the compiler will instrument integer division instructions
    /// to capture the right argument of division.
    #[arg(long)]
    pub trace_div: bool,

    /// Enables `sanitizer-coverage-trace-geps` LLVM instrumentation
    ///
    /// When set to `true`, instruments GetElementPtr (GEP) instructions to track
    /// pointer arithmetic operations to capture array indices.
    #[arg(long)]
    pub trace_gep: bool,

    /// Disable transformation of if-statements into `cmov` instructions (when this
    /// happens, we get no coverage feedback for that branch). Default setting is true.
    /// This is done by setting the `-simplifycfg-branch-fold-threshold=0` LLVM arg.
    ///
    /// For example, in the following program shows the default coverage feedback when
    /// compiled with `-Copt-level=3`:
    ///
    /// mark_covered(1); // mark edge 1 as covered
    /// let mut res = 1;
    /// if x > 5 && y < 6 {
    ///    res = 2;
    /// }
    ///
    /// With `disable_branch_folding` enabled, the code compiles to be equivalent to:
    ///
    /// mark_covered(1);
    /// let mut res = 1;
    /// if x > 5 {
    ///     mark_covered(2);
    ///     if y < 6 {
    ///         mark_covered(3);
    ///         res = 2;
    ///     }
    /// }
    ///
    /// Note, that in the second program, there are now 2 new coverage feedback points,
    /// and the fuzzer can store an input to the corpus at each condition that it passes;
    /// giving it a better chance of producing an input that reaches `res = 2;`.
    //
    // `Option<Option<bool>>` gives an optional argument with an optional value.
    #[arg(long)]
    pub disable_branch_folding: Option<Option<bool>>,

    /// Disable the inclusion of the `/include:main` MSVC linker argument
    ///
    /// The purpose of `/include:main` is to force the MSVC linker to include an
    /// external reference to the symbol `main`, such that fuzzing targets built
    /// on Windows are able to find LibFuzzer's `main` function.
    ///
    /// In certain corner cases, users may prefer to *not* build with this
    /// argument. One such example: if a user is intending to build and fuzz a
    /// Windows DLL, they would likely choose to enable this flag, to prevent
    /// the DLL from having an extern `main` reference added to it. (DLLs/shared
    /// libraries should not have any reference to `main`.)
    #[arg(long)]
    pub no_include_main_msvc: bool,
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
            codegen_units: None,
            strip_dead_code: None,
            no_cfg_fuzzing: false,
            no_trace_compares: false,
            trace_div: false,
            trace_gep: false,
            disable_branch_folding: None,
            no_include_main_msvc: false,
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
