use crate::{options::BuildOptions, project::FuzzProject, RunCommand};
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
pub struct Cmin {
    #[command(flatten)]
    pub build: BuildOptions,

    /// Name of the fuzz target
    pub target: String,

    #[arg()]
    /// The corpus directory to minify into
    pub corpus: Option<PathBuf>,

    #[arg(last(true))]
    /// Additional libFuzzer arguments passed through to the binary
    pub args: Vec<String>,
}

impl RunCommand for Cmin {
    fn run_command(&mut self) -> Result<()> {
        let project = FuzzProject::new(self.build.fuzz_dir_wrapper.get_manifest_path())?;
        project.exec_cmin(self)
    }
}
