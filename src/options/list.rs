use crate::{FuzzProject, RunCommand};
use anyhow::Result;
use structopt::StructOpt;

#[derive(Clone, Debug, StructOpt)]
pub struct List {}

impl RunCommand for List {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new()?;
        project.list_targets()
    }
}
