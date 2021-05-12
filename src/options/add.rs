use crate::{options::FuzzDirWrapper, project::FuzzProject, RunCommand};
use anyhow::Result;
use structopt::StructOpt;

#[derive(Clone, Debug, StructOpt)]
pub struct Add {
    #[structopt(flatten)]
    pub fuzz_dir_wrapper: FuzzDirWrapper,

    /// Name of the new fuzz target
    pub target: String,
}

impl RunCommand for Add {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new(self.fuzz_dir_wrapper.fuzz_dir.to_owned())?;
        project.add_target(self)
    }
}
