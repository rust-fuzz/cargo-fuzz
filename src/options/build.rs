use crate::{options::BuildOptions, project::FuzzProject, RunCommand};
use anyhow::Result;
use structopt::StructOpt;

#[derive(Clone, Debug, StructOpt)]
pub struct Build {
    #[structopt(flatten)]
    pub build: BuildOptions,

    /// Name of the fuzz target to build, or build all targets if not supplied
    pub target: Option<String>,
}

impl RunCommand for Build {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::find_existing()?;
        project.exec_build(&self.build, self.target.as_deref().map(|s| s))
    }
}
