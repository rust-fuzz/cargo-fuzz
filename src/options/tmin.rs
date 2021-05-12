use crate::{
    options::{BuildOptions, FuzzDirWrapper},
    project::FuzzProject,
    RunCommand,
};
use anyhow::Result;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Clone, Debug, StructOpt)]
pub struct Tmin {
    #[structopt(flatten)]
    pub build: BuildOptions,

    #[structopt(flatten)]
    pub fuzz_dir_wrapper: FuzzDirWrapper,

    /// Name of the fuzz target
    pub target: String,

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
    pub runs: u32,

    #[structopt(parse(from_os_str))]
    /// Path to the failing test case to be minimized
    pub test_case: PathBuf,

    #[structopt(last(true))]
    /// Additional libFuzzer arguments passed through to the binary
    pub args: Vec<String>,
}

impl RunCommand for Tmin {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new(self.fuzz_dir_wrapper.fuzz_dir.to_owned())?;
        project.exec_tmin(self)
    }
}
