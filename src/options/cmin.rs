use crate::{options::BuildOptions, project::FuzzProject, RunCommand};
use anyhow::Result;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Clone, Debug, StructOpt)]
pub struct Cmin {
    #[structopt(flatten)]
    pub build: BuildOptions,

    /// Name of the fuzz target
    pub target: String,

    #[structopt(parse(from_os_str))]
    /// The corpus directory to minify into
    pub corpus: Option<PathBuf>,

    #[structopt(last(true))]
    /// Additional libFuzzer arguments passed through to the binary
    pub args: Vec<String>,
}

impl RunCommand for Cmin {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::find_existing()?;
        project.exec_cmin(self)
    }
}
