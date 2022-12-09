use crate::{options::FuzzDirWrapper, project::FuzzProject, RunCommand};
use anyhow::Result;
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct List {
    #[command(flatten)]
    pub fuzz_dir_wrapper: FuzzDirWrapper,
}

impl RunCommand for List {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new(self.fuzz_dir_wrapper.fuzz_dir.to_owned())?;
        project.list_targets()
    }
}
