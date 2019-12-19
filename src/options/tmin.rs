use crate::{options::BuildOptions, project::FuzzProject, RunCommand};
use anyhow::Result;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Clone, Debug, StructOpt)]
pub struct Tmin {
    #[structopt(flatten)]
    pub build: BuildOptions,

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
}

impl RunCommand for Tmin {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::find_existing()?;
        project.exec_tmin(self)
    }
}
