use crate::project::{FuzzProject, Manifest};
use crate::{options::FuzzDirWrapper, RunCommand};
use anyhow::Result;
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Add {
    #[command(flatten)]
    pub fuzz_dir_wrapper: FuzzDirWrapper,

    /// Name of the new fuzz target
    pub target: String,
}

impl RunCommand for Add {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new(self.fuzz_dir_wrapper.fuzz_dir.to_owned())?;
        let manifest = Manifest::parse()?;
        project.add_target(self, &manifest)
    }
}
