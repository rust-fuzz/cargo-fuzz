use crate::options::{self, BuildMode, BuildOptions, Sanitizer};
use crate::utils::default_target;
use anyhow::{anyhow, bail, Context, Result};
use std::collections::HashSet;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{
    env, ffi, fs,
    process::{Command, Stdio},
    time,
};

const DEFAULT_FUZZ_DIR: &str = "fuzz";

pub struct FuzzProject {
    /// The project with fuzz targets
    fuzz_dir: PathBuf,
    /// The project being fuzzed
    project_dir: PathBuf,
    targets: Vec<String>,
}

impl FuzzProject {
    /// Creates a new instance.
    //
    /// Find an existing `cargo fuzz` project by starting at the current
    /// directory and walking up the filesystem.
    ///
    /// If `fuzz_dir_opt` is `None`, returns a new instance with the default fuzz project
    /// path.
    pub fn new(fuzz_dir_opt: Option<PathBuf>) -> Result<Self> {
        let mut project = Self::manage_initial_instance(fuzz_dir_opt)?;
        let manifest = project.manifest()?;
        if !is_fuzz_manifest(&manifest) {
            bail!(
                "manifest `{}` does not look like a cargo-fuzz manifest. \
                 Add following lines to override:\n\
                 [package.metadata]\n\
                 cargo-fuzz = true",
                project.manifest_path().display()
            );
        }
        project.targets = collect_targets(&manifest);
        Ok(project)
    }

    /// Creates the fuzz project structure and returns a new instance.
    ///
    /// This will not clone libfuzzer-sys.
    /// Similar to `FuzzProject::new`, the fuzz directory will depend on `fuzz_dir_opt`.
    pub fn init(init: &options::Init, fuzz_dir_opt: Option<PathBuf>) -> Result<Self> {
        let project = Self::manage_initial_instance(fuzz_dir_opt)?;
        let fuzz_project = project.fuzz_dir();
        let root_project_manifest_path = project.project_dir.join("Cargo.toml");
        let manifest = Manifest::parse(&root_project_manifest_path)?;

        // TODO: check if the project is already initialized
        fs::create_dir(fuzz_project)
            .with_context(|| format!("failed to create directory {}", fuzz_project.display()))?;

        let fuzz_targets_dir = fuzz_project.join(crate::FUZZ_TARGETS_DIR);
        fs::create_dir(&fuzz_targets_dir).with_context(|| {
            format!("failed to create directory {}", fuzz_targets_dir.display())
        })?;

        let cargo_toml = fuzz_project.join("Cargo.toml");
        let mut cargo = fs::File::create(&cargo_toml)
            .with_context(|| format!("failed to create {}", cargo_toml.display()))?;
        cargo
            .write_fmt(toml_template!(manifest.crate_name, manifest.edition))
            .with_context(|| format!("failed to write to {}", cargo_toml.display()))?;

        let gitignore = fuzz_project.join(".gitignore");
        let mut ignore = fs::File::create(&gitignore)
            .with_context(|| format!("failed to create {}", gitignore.display()))?;
        ignore
            .write_fmt(gitignore_template!())
            .with_context(|| format!("failed to write to {}", gitignore.display()))?;

        project
            .create_target_template(&init.target, &manifest)
            .with_context(|| {
                format!(
                    "could not create template file for target {:?}",
                    init.target
                )
            })?;
        Ok(project)
    }

    pub fn list_targets(&self) -> Result<()> {
        for bin in &self.targets {
            println!("{}", bin);
        }
        Ok(())
    }

    /// Create a new fuzz target.
    pub fn add_target(&self, add: &options::Add, manifest: &Manifest) -> Result<()> {
        // Create corpus and artifact directories for the newly added target
        self.corpus_for(&add.target)?;
        self.artifacts_for(&add.target)?;
        self.create_target_template(&add.target, manifest)
            .with_context(|| format!("could not add target {:?}", add.target))
    }

    /// Add a new fuzz target script with a given name
    fn create_target_template(&self, target: &str, manifest: &Manifest) -> Result<()> {
        let target_path = self.target_path(target);

        // If the user manually created a fuzz project, but hasn't created any
        // targets yet, the `fuzz_targets` directory might not exist yet,
        // despite a `fuzz/Cargo.toml` manifest with the `metadata.cargo-fuzz`
        // key present. Make sure it does exist.
        fs::create_dir_all(self.fuzz_targets_dir())
            .context("ensuring that `fuzz_targets` directory exists failed")?;

        let mut script = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target_path)
            .with_context(|| format!("could not create target script file at {:?}", target_path))?;
        script.write_fmt(target_template!(manifest.edition))?;

        let mut cargo = fs::OpenOptions::new()
            .append(true)
            .open(self.manifest_path())?;
        Ok(cargo.write_fmt(toml_bin_template!(target))?)
    }

    fn cargo(&self, subcommand: &str, build: &BuildOptions) -> Result<Command> {
        let mut cmd = Command::new("cargo");
        cmd.arg(subcommand)
            .arg("--manifest-path")
            .arg(self.manifest_path())
            // --target=<TARGET> won't pass rustflags to build scripts
            .arg("--target")
            .arg(&build.triple);
        // we default to release mode unless debug mode is explicitly requested
        if !build.dev {
            cmd.arg("--release");
        }
        if build.verbose {
            cmd.arg("--verbose");
        }
        if build.no_default_features {
            cmd.arg("--no-default-features");
        }
        if build.all_features {
            cmd.arg("--all-features");
        }
        if let Some(ref features) = build.features {
            cmd.arg("--features").arg(features);
        }
        for flag in &build.unstable_flags {
            cmd.arg("-Z").arg(flag);
        }
        if let Sanitizer::Memory = build.sanitizer {
            cmd.arg("-Z").arg("build-std");
        } else if build.build_std.unwrap_or(true) && !build.coverage {
            cmd.arg("-Z").arg("build-std");
        }

        let mut rustflags: String = "-Cpasses=sancov-module \
                                     -Cllvm-args=-sanitizer-coverage-level=4 \
                                     -Cllvm-args=-sanitizer-coverage-inline-8bit-counters \
                                     -Cllvm-args=-sanitizer-coverage-pc-table"
            .to_owned();

        if !build.no_trace_compares {
            rustflags.push_str(" -Cllvm-args=-sanitizer-coverage-trace-compares");
        }

        if !build.no_cfg_fuzzing {
            rustflags.push_str(" --cfg fuzzing");
        }

        if !build.strip_dead_code {
            rustflags.push_str(" -Clink-dead-code");
        }

        if build.coverage {
            rustflags.push_str(" -Cinstrument-coverage");
        }

        match build.sanitizer {
            Sanitizer::None => {}
            Sanitizer::Memory => {
                // Memory sanitizer requires more flags to function than others:
                // https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html#memorysanitizer
                rustflags.push_str(" -Zsanitizer=memory -Zsanitizer-memory-track-origins")
            }
            _ => rustflags.push_str(&format!(
                " -Zsanitizer={sanitizer}",
                sanitizer = build.sanitizer
            )),
        }
        if build.triple.contains("-linux-") {
            rustflags.push_str(" -Cllvm-args=-sanitizer-coverage-stack-depth");
        }
        if !build.release || build.debug_assertions {
            rustflags.push_str(" -Cdebug-assertions");
        }
        if build.triple.contains("-msvc") {
            // The entrypoint is in the bundled libfuzzer rlib, this gets the linker to find it.
            rustflags.push_str(" -Clink-arg=/include:main");
        }

        // If release mode is enabled then we force 1 CGU to be used in rustc.
        // This will result in slower compilations but it looks like the sancov
        // passes otherwise add `notEligibleToImport` annotations to functions
        // in LLVM IR, meaning that *nothing* can get imported with ThinLTO.
        // This means that in release mode, where ThinLTO is critical for
        // performance, we're taking a huge hit relative to actual release mode.
        // Local tests have once showed this to be a ~3x faster runtime where
        // otherwise functions like `Vec::as_ptr` aren't inlined.
        if !build.dev {
            rustflags.push_str(" -C codegen-units=1");
        }

        if let Ok(other_flags) = env::var("RUSTFLAGS") {
            rustflags.push(' ');
            rustflags.push_str(&other_flags);
        }
        cmd.env("RUSTFLAGS", rustflags);

        // For asan and tsan we have default options. Merge them to the given
        // options, so users can still provide their own options to e.g. disable
        // the leak sanitizer.  Options are colon-separated.
        match build.sanitizer {
            Sanitizer::Address => {
                let mut asan_opts = env::var("ASAN_OPTIONS").unwrap_or_default();
                if !asan_opts.is_empty() {
                    asan_opts.push(':');
                }
                asan_opts.push_str("detect_odr_violation=0");
                cmd.env("ASAN_OPTIONS", asan_opts);
            }

            Sanitizer::Thread => {
                let mut tsan_opts = env::var("TSAN_OPTIONS").unwrap_or_default();
                if !tsan_opts.is_empty() {
                    tsan_opts.push(':');
                }
                tsan_opts.push_str("report_signal_unsafe=0");
                cmd.env("TSAN_OPTIONS", tsan_opts);
            }

            _ => {}
        }

        Ok(cmd)
    }

    fn cargo_run(&self, build: &options::BuildOptions, fuzz_target: &str) -> Result<Command> {
        let mut cmd = self.cargo("run", build)?;
        cmd.arg("--bin").arg(fuzz_target);

        if let Some(target_dir) = &build.target_dir {
            cmd.arg("--target-dir").arg(target_dir);
        }

        let mut artifact_arg = ffi::OsString::from("-artifact_prefix=");
        artifact_arg.push(self.artifacts_for(fuzz_target)?);
        cmd.arg("--").arg(artifact_arg);

        Ok(cmd)
    }

    // note: never returns Ok(None) if build.coverage is true
    fn target_dir(&self, build: &options::BuildOptions) -> Result<Option<PathBuf>> {
        // Use the user-provided target directory, if provided. Otherwise if building for coverage,
        // use the coverage directory
        if let Some(target_dir) = build.target_dir.as_ref() {
            return Ok(Some(PathBuf::from(target_dir)));
        } else if build.coverage {
            // To ensure that fuzzing and coverage-output generation can run in parallel, we
            // produce a separate binary for the coverage command.
            let current_dir = env::current_dir()?;
            Ok(Some(
                current_dir
                    .join("target")
                    .join(default_target())
                    .join("coverage"),
            ))
        } else {
            Ok(None)
        }
    }

    pub fn exec_build(
        &self,
        mode: options::BuildMode,
        build: &options::BuildOptions,
        fuzz_target: Option<&str>,
    ) -> Result<()> {
        let cargo_subcommand = match mode {
            options::BuildMode::Build => "build",
            options::BuildMode::Check => "check",
        };
        let mut cmd = self.cargo(cargo_subcommand, build)?;

        if let Some(fuzz_target) = fuzz_target {
            cmd.arg("--bin").arg(fuzz_target);
        } else {
            cmd.arg("--bins");
        }

        if let Some(target_dir) = self.target_dir(&build)? {
            cmd.arg("--target-dir").arg(target_dir);
        }

        let status = cmd
            .status()
            .with_context(|| format!("failed to execute: {:?}", cmd))?;
        if !status.success() {
            bail!("failed to build fuzz script: {:?}", cmd);
        }

        Ok(())
    }

    fn get_artifacts_since(
        &self,
        target: &str,
        since: &time::SystemTime,
    ) -> Result<HashSet<PathBuf>> {
        let mut artifacts = HashSet::new();

        let artifacts_dir = self.artifacts_for(target)?;

        for entry in fs::read_dir(&artifacts_dir).with_context(|| {
            format!(
                "failed to read directory entries of {}",
                artifacts_dir.display()
            )
        })? {
            let entry = entry.with_context(|| {
                format!(
                    "failed to read directory entry inside {}",
                    artifacts_dir.display()
                )
            })?;

            let metadata = entry
                .metadata()
                .context("failed to read artifact metadata")?;
            let modified = metadata
                .modified()
                .context("failed to get artifact modification time")?;
            if !metadata.is_file() || modified <= *since {
                continue;
            }

            artifacts.insert(entry.path());
        }

        Ok(artifacts)
    }

    fn run_fuzz_target_debug_formatter(
        &self,
        build: &BuildOptions,
        target: &str,
        artifact: &Path,
    ) -> Result<String> {
        let debug_output = tempfile::NamedTempFile::new().context("failed to create temp file")?;

        let mut cmd = self.cargo_run(build, target)?;
        cmd.stdin(Stdio::null());
        cmd.env("RUST_LIBFUZZER_DEBUG_PATH", debug_output.path());
        cmd.arg(artifact);

        let output = cmd
            .output()
            .with_context(|| format!("failed to run command: {:?}", cmd))?;

        if !output.status.success() {
            bail!(
                "Fuzz target '{target}' exited with failure when attempting to \
                 debug formatting an interesting input that we discovered!\n\n\
                 Artifact: {artifact}\n\n\
                 Command: {cmd:?}\n\n\
                 Status: {status}\n\n\
                 === stdout ===\n\
                 {stdout}\n\n\
                 === stderr ===\n\
                 {stderr}",
                target = target,
                status = output.status,
                cmd = cmd,
                artifact = artifact.display(),
                stdout = String::from_utf8_lossy(&output.stdout),
                stderr = String::from_utf8_lossy(&output.stderr),
            );
        }

        let debug = fs::read_to_string(&debug_output).context("failed to read temp file")?;
        Ok(debug)
    }

    /// Prints the debug output of an input test case
    pub fn debug_fmt_input(&self, debugfmt: &options::Fmt) -> Result<()> {
        if !debugfmt.input.exists() {
            bail!(
                "Input test case does not exist: {}",
                debugfmt.input.display()
            );
        }

        let debug = self
            .run_fuzz_target_debug_formatter(&debugfmt.build, &debugfmt.target, &debugfmt.input)
            .with_context(|| {
                format!(
                    "failed to run `cargo fuzz fmt` on input: {}",
                    debugfmt.input.display()
                )
            })?;

        eprintln!("\nOutput of `std::fmt::Debug`:\n");
        for l in debug.lines() {
            eprintln!("{}", l);
        }

        Ok(())
    }

    /// Fuzz a given fuzz target
    pub fn exec_fuzz(&self, run: &options::Run) -> Result<()> {
        self.exec_build(BuildMode::Build, &run.build, Some(&run.target))?;
        let mut cmd = self.cargo_run(&run.build, &run.target)?;

        for arg in &run.args {
            cmd.arg(arg);
        }

        if !run.corpus.is_empty() {
            for corpus in &run.corpus {
                cmd.arg(corpus);
            }
        } else {
            cmd.arg(self.corpus_for(&run.target)?);
        }

        if run.jobs != 1 {
            cmd.arg(format!("-fork={}", run.jobs));
        }

        // When libfuzzer finds failing inputs, those inputs will end up in the
        // artifacts directory. To easily filter old artifacts from new ones,
        // get the current time, and then later we only consider files modified
        // after now.
        let before_fuzzing = time::SystemTime::now();

        let mut child = cmd
            .spawn()
            .with_context(|| format!("failed to spawn command: {:?}", cmd))?;
        let status = child
            .wait()
            .with_context(|| format!("failed to wait on child process for command: {:?}", cmd))?;
        if status.success() {
            return Ok(());
        }

        // Get and print the `Debug` formatting of any new artifacts, along with
        // tips about how to reproduce failures and/or minimize test cases.

        let new_artifacts = self.get_artifacts_since(&run.target, &before_fuzzing)?;

        for artifact in new_artifacts {
            // To make the artifact a little easier to read, strip the current
            // directory prefix when possible.
            let artifact = strip_current_dir_prefix(&artifact);

            eprintln!("\n{:─<80}", "");
            eprintln!("\nFailing input:\n\n\t{}\n", artifact.display());

            // Note: ignore errors when running the debug formatter. This most
            // likely just means that we're dealing with a fuzz target that uses
            // an older version of the libfuzzer crate, and doesn't support
            // `RUST_LIBFUZZER_DEBUG_PATH`.
            if let Ok(debug) =
                self.run_fuzz_target_debug_formatter(&run.build, &run.target, artifact)
            {
                eprintln!("Output of `std::fmt::Debug`:\n");
                for l in debug.lines() {
                    eprintln!("\t{}", l);
                }
                eprintln!();
            }

            let fuzz_dir = if self.fuzz_dir_is_default_path() {
                String::new()
            } else {
                format!(" --fuzz-dir {}", self.fuzz_dir().display())
            };

            eprintln!(
                "Reproduce with:\n\n\tcargo fuzz run{fuzz_dir}{options} {target} {artifact}\n",
                fuzz_dir = &fuzz_dir,
                options = &run.build,
                target = &run.target,
                artifact = artifact.display()
            );
            eprintln!(
                "Minimize test case with:\n\n\tcargo fuzz tmin{fuzz_dir}{options} {target} {artifact}\n",
                fuzz_dir = &fuzz_dir,
                options = &run.build,
                target = &run.target,
                artifact = artifact.display()
            );
        }

        eprintln!("{:─<80}\n", "");
        bail!("Fuzz target exited with {}", status)
    }

    pub fn exec_tmin(&self, tmin: &options::Tmin) -> Result<()> {
        self.exec_build(BuildMode::Build, &tmin.build, Some(&tmin.target))?;
        let mut cmd = self.cargo_run(&tmin.build, &tmin.target)?;
        cmd.arg("-minimize_crash=1")
            .arg(format!("-runs={}", tmin.runs))
            .arg(&tmin.test_case);

        for arg in &tmin.args {
            cmd.arg(arg);
        }

        let before_tmin = time::SystemTime::now();

        let mut child = cmd
            .spawn()
            .with_context(|| format!("failed to spawn command: {:?}", cmd))?;
        let status = child
            .wait()
            .with_context(|| format!("failed to wait on child process for command: {:?}", cmd))?;
        if !status.success() {
            eprintln!("\n{:─<80}\n", "");
            return Err(anyhow!("Command `{:?}` exited with {}", cmd, status)).with_context(|| {
                "Test case minimization failed.\n\
                 \n\
                 Usually this isn't a hard error, and just means that libfuzzer\n\
                 doesn't know how to minimize the test case any further while\n\
                 still reproducing the original crash.\n\
                 \n\
                 See the logs above for details."
            });
        }

        // Find and display the most recently modified artifact, which is
        // presumably the result of minification. Yeah, this is a little hacky,
        // but it seems to work. I don't want to parse libfuzzer's stderr output
        // and hope it never changes.
        let minimized_artifact = self
            .get_artifacts_since(&tmin.target, &before_tmin)?
            .into_iter()
            .max_by_key(|a| {
                a.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(time::SystemTime::UNIX_EPOCH)
            });

        if let Some(artifact) = minimized_artifact {
            let artifact = strip_current_dir_prefix(&artifact);

            eprintln!("\n{:─<80}\n", "");
            eprintln!("Minimized artifact:\n\n\t{}\n", artifact.display());

            // Note: ignore errors when running the debug formatter. This most
            // likely just means that we're dealing with a fuzz target that uses
            // an older version of the libfuzzer crate, and doesn't support
            // `RUST_LIBFUZZER_DEBUG_PATH`.
            if let Ok(debug) =
                self.run_fuzz_target_debug_formatter(&tmin.build, &tmin.target, artifact)
            {
                eprintln!("Output of `std::fmt::Debug`:\n");
                for l in debug.lines() {
                    eprintln!("\t{}", l);
                }
                eprintln!();
            }

            let fuzz_dir = if self.fuzz_dir_is_default_path() {
                String::new()
            } else {
                format!(" --fuzz-dir {}", self.fuzz_dir().display())
            };

            eprintln!(
                "Reproduce with:\n\n\tcargo fuzz run{fuzz_dir}{options} {target} {artifact}\n",
                fuzz_dir = &fuzz_dir,
                options = &tmin.build,
                target = &tmin.target,
                artifact = artifact.display()
            );
        }

        Ok(())
    }

    pub fn exec_cmin(&self, cmin: &options::Cmin) -> Result<()> {
        self.exec_build(BuildMode::Build, &cmin.build, Some(&cmin.target))?;
        let mut cmd = self.cargo_run(&cmin.build, &cmin.target)?;

        for arg in &cmin.args {
            cmd.arg(arg);
        }

        let corpus = if let Some(corpus) = cmin.corpus.clone() {
            corpus
        } else {
            self.corpus_for(&cmin.target)?
        };
        let corpus = corpus
            .to_str()
            .ok_or_else(|| anyhow!("corpus must be valid unicode"))?
            .to_owned();

        let tmp = tempfile::TempDir::new_in(self.fuzz_dir())?;
        let tmp_corpus = tmp.path().join("corpus");
        fs::create_dir(&tmp_corpus)?;

        cmd.arg("-merge=1").arg(&tmp_corpus).arg(&corpus);

        // Spawn cmd in child process instead of exec-ing it
        let status = cmd
            .status()
            .with_context(|| format!("could not execute command: {:?}", cmd))?;
        if status.success() {
            // move corpus directory into tmp to auto delete it
            fs::rename(&corpus, tmp.path().join("old"))?;
            fs::rename(tmp.path().join("corpus"), corpus)?;
        } else {
            println!("Failed to minimize corpus: {}", status);
        }

        Ok(())
    }

    /// Produce coverage information for a given corpus
    pub fn exec_coverage(self, coverage: &options::Coverage) -> Result<()> {
        // Build project with source-based coverage generation enabled.
        self.exec_build(BuildMode::Build, &coverage.build, Some(&coverage.target))?;

        // Retrieve corpus directories.
        let corpora = if coverage.corpus.is_empty() {
            vec![self.corpus_for(&coverage.target)?]
        } else {
            coverage
                .corpus
                .iter()
                .map(|name| Path::new(name).to_path_buf())
                .collect()
        };

        // Collect the (non-directory) readable input files from the corpora.
        let files_and_dirs = corpora.iter().flat_map(fs::read_dir).flatten().flatten();
        let mut readable_input_files = files_and_dirs
            .filter(|file| match file.file_type() {
                Ok(ft) => ft.is_file(),
                _ => false,
            })
            .peekable();
        if readable_input_files.peek().is_none() {
            bail!(
                "The corpus does not contain program-input files. \
                 Coverage information requires existing input files. \
                 Try running the fuzzer first (`cargo fuzz run ...`) to generate a corpus, \
                 or provide a nonempty corpus directory."
            )
        }

        let (coverage_out_raw_dir, coverage_out_file) = self.coverage_for(&coverage.target)?;

        for corpus in corpora.iter() {
            // _tmp_dir is deleted when it goes of of scope.
            let (mut cmd, _tmp_dir) =
                self.create_coverage_cmd(coverage, &coverage_out_raw_dir, &corpus.as_path())?;
            eprintln!("Generating coverage data for corpus {:?}", corpus);
            let status = cmd
                .status()
                .with_context(|| format!("Failed to run command: {:?}", cmd))?;
            if !status.success() {
                Err(anyhow!(
                    "Command exited with failure status {}: {:?}",
                    status,
                    cmd
                ))
                .context("Failed to generage coverage data")?;
            }
        }
        self.merge_coverage(&coverage_out_raw_dir, &coverage_out_file)?;

        Ok(())
    }

    fn create_coverage_cmd(
        &self,
        coverage: &options::Coverage,
        coverage_dir: &Path,
        corpus_dir: &Path,
    ) -> Result<(Command, tempfile::TempDir)> {
        let bin_path = {
            let profile_subdir = if coverage.build.dev {
                "debug"
            } else {
                "release"
            };

            let target_dir = self
                .target_dir(&coverage.build)?
                .expect("target dir for coverage command should never be None");
            target_dir
                .join(&coverage.build.triple)
                .join(profile_subdir)
                .join(&coverage.target)
        };

        let mut cmd = Command::new(bin_path);

        // Raw coverage data will be saved in `coverage/<target>` directory.
        let corpus_dir_name = corpus_dir
            .file_name()
            .and_then(|x| x.to_str())
            .with_context(|| format!("Invalid corpus directory: {:?}", corpus_dir))?;
        cmd.env(
            "LLVM_PROFILE_FILE",
            coverage_dir.join(format!("default-{}.profraw", corpus_dir_name)),
        );
        cmd.arg("-merge=1");
        let dummy_corpus = tempfile::tempdir()?;
        cmd.arg(dummy_corpus.path());
        cmd.arg(corpus_dir);

        for arg in &coverage.args {
            cmd.arg(arg);
        }

        Ok((cmd, dummy_corpus))
    }

    fn merge_coverage(&self, profdata_raw_path: &Path, profdata_out_path: &Path) -> Result<()> {
        let mut profdata_path = rustlib()?;
        profdata_path.push(format!("llvm-profdata{}", env::consts::EXE_SUFFIX));
        let mut merge_cmd = Command::new(profdata_path);
        merge_cmd.arg("merge").arg("-sparse");
        merge_cmd.arg(profdata_raw_path);
        merge_cmd.arg("-o").arg(profdata_out_path);

        eprintln!("Merging raw coverage data...");
        let status = merge_cmd
            .status()
            .with_context(|| format!("Failed to run command: {:?}", merge_cmd))
            .with_context(|| "Merging raw coverage files failed.\n\
                              \n\
                              Do you have LLVM coverage tools installed?\n\
                              https://doc.rust-lang.org/rustc/instrument-coverage.html#installing-llvm-coverage-tools")?;
        if !status.success() {
            Err(anyhow!(
                "Command exited with failure status {}: {:?}",
                status,
                merge_cmd
            ))
            .context("Merging raw coverage files failed")?;
        }

        if profdata_out_path.exists() {
            eprintln!("Coverage data merged and saved in {:?}.", profdata_out_path);
            Ok(())
        } else {
            bail!("Coverage data could not be merged.")
        }
    }

    pub(crate) fn fuzz_dir(&self) -> &Path {
        &self.fuzz_dir
    }

    fn manifest_path(&self) -> PathBuf {
        self.fuzz_dir().join("Cargo.toml")
    }

    /// Returns paths to the `coverage/<target>/raw` directory and `coverage/<target>/coverage.profdata` file.
    fn coverage_for(&self, target: &str) -> Result<(PathBuf, PathBuf)> {
        let mut coverage_data = self.fuzz_dir().to_owned();
        coverage_data.push("coverage");
        coverage_data.push(target);
        let mut coverage_raw = coverage_data.clone();
        coverage_data.push("coverage.profdata");
        coverage_raw.push("raw");
        fs::create_dir_all(&coverage_raw).with_context(|| {
            format!("could not make a coverage directory at {:?}", coverage_raw)
        })?;
        Ok((coverage_raw, coverage_data))
    }

    fn corpus_for(&self, target: &str) -> Result<PathBuf> {
        let mut p = self.fuzz_dir().to_owned();
        p.push("corpus");
        p.push(target);
        fs::create_dir_all(&p)
            .with_context(|| format!("could not make a corpus directory at {:?}", p))?;
        Ok(p)
    }

    fn artifacts_for(&self, target: &str) -> Result<PathBuf> {
        let mut p = self.fuzz_dir().to_owned();
        p.push("artifacts");
        p.push(target);

        // This adds a trailing slash, which is necessary for libFuzzer, because
        // it does simple string concatenation when joining paths.
        p.push("");

        fs::create_dir_all(&p)
            .with_context(|| format!("could not make a artifact directory at {:?}", p))?;

        Ok(p)
    }

    fn fuzz_targets_dir(&self) -> PathBuf {
        let mut root = self.fuzz_dir().to_owned();
        if root.join(crate::FUZZ_TARGETS_DIR_OLD).exists() {
            println!(
                "warning: The `fuzz/fuzzers/` directory has renamed to `fuzz/fuzz_targets/`. \
                 Please rename the directory as such. This will become a hard error in the \
                 future."
            );
            root.push(crate::FUZZ_TARGETS_DIR_OLD);
        } else {
            root.push(crate::FUZZ_TARGETS_DIR);
        }
        root
    }

    fn target_path(&self, target: &str) -> PathBuf {
        let mut root = self.fuzz_targets_dir();
        root.push(target);
        root.set_extension("rs");
        root
    }

    fn manifest(&self) -> Result<toml::Value> {
        let filename = self.manifest_path();
        let mut file = fs::File::open(&filename)
            .with_context(|| format!("could not read the manifest file: {}", filename.display()))?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        toml::from_slice(&data).with_context(|| {
            format!(
                "could not decode the manifest file at {}",
                filename.display()
            )
        })
    }

    // If `fuzz_dir_opt` is `None`, returns a new instance with the default fuzz project
    // path. Otherwise, returns a new instance with the inner content of `fuzz_dir_opt`.
    fn manage_initial_instance(fuzz_dir_opt: Option<PathBuf>) -> Result<Self> {
        let project_dir = find_package()?;
        let fuzz_dir = if let Some(el) = fuzz_dir_opt {
            el
        } else {
            project_dir.join(DEFAULT_FUZZ_DIR)
        };
        Ok(FuzzProject {
            fuzz_dir,
            project_dir,
            targets: Vec::new(),
        })
    }

    fn fuzz_dir_is_default_path(&self) -> bool {
        self.fuzz_dir.ends_with(DEFAULT_FUZZ_DIR)
    }
}

fn sysroot() -> Result<String> {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = Command::new(rustc).arg("--print").arg("sysroot").output()?;
    // Note: We must trim() to remove the `\n` from the end of stdout
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn rustlib() -> Result<PathBuf> {
    let sysroot = sysroot()?;
    let mut pathbuf = PathBuf::from(sysroot);
    pathbuf.push("lib");
    pathbuf.push("rustlib");
    pathbuf.push(rustc_version::version_meta()?.host);
    pathbuf.push("bin");
    Ok(pathbuf)
}

fn collect_targets(value: &toml::Value) -> Vec<String> {
    let bins = value
        .as_table()
        .and_then(|v| v.get("bin"))
        .and_then(toml::Value::as_array);
    let mut bins = if let Some(bins) = bins {
        bins.iter()
            .map(|bin| {
                bin.as_table()
                    .and_then(|v| v.get("name"))
                    .and_then(toml::Value::as_str)
            })
            .filter_map(|name| name.map(String::from))
            .collect()
    } else {
        Vec::new()
    };
    // Always sort them, so that we have deterministic output.
    bins.sort();
    bins
}

pub struct Manifest {
    crate_name: String,
    edition: Option<String>,
}

impl Manifest {
    pub fn parse(path: &Path) -> Result<Self> {
        let contents = fs::read(path)?;
        let value: toml::Value = toml::from_slice(&contents)?;
        let package = value
            .as_table()
            .and_then(|v| v.get("package"))
            .and_then(toml::Value::as_table);
        let crate_name = package
            .and_then(|v| v.get("name"))
            .and_then(toml::Value::as_str)
            .with_context(|| anyhow!("{} (package.name) is malformed", path.display()))?
            .to_owned();
        let edition = package
            .expect("can't be None at this point")
            .get("edition")
            .map(|v| match v.as_str() {
                Some(s) => Ok(s.to_owned()),
                None => bail!("{} (package.edition) is malformed", path.display()),
            })
            .transpose()?;
        Ok(Manifest {
            crate_name,
            edition,
        })
    }
}

fn is_fuzz_manifest(value: &toml::Value) -> bool {
    let is_fuzz = value
        .as_table()
        .and_then(|v| v.get("package"))
        .and_then(toml::Value::as_table)
        .and_then(|v| v.get("metadata"))
        .and_then(toml::Value::as_table)
        .and_then(|v| v.get("cargo-fuzz"))
        .and_then(toml::Value::as_bool);
    is_fuzz == Some(true)
}

/// Returns the path for the first found non-fuzz Cargo package
fn find_package() -> Result<PathBuf> {
    let mut dir = env::current_dir()?;
    let mut data = Vec::new();
    loop {
        let manifest_path = dir.join("Cargo.toml");
        match fs::File::open(&manifest_path) {
            Err(_) => {}
            Ok(mut f) => {
                data.clear();
                f.read_to_end(&mut data)
                    .with_context(|| format!("failed to read {}", manifest_path.display()))?;
                let value: toml::Value = toml::from_slice(&data).with_context(|| {
                    format!(
                        "could not decode the manifest file at {}",
                        manifest_path.display()
                    )
                })?;
                if !is_fuzz_manifest(&value) {
                    // Not a cargo-fuzz project => must be a proper cargo project :)
                    return Ok(dir);
                }
            }
        }
        if !dir.pop() {
            break;
        }
    }
    bail!("could not find a cargo project")
}

fn strip_current_dir_prefix(path: &Path) -> &Path {
    env::current_dir()
        .ok()
        .and_then(|curdir| path.strip_prefix(curdir).ok())
        .unwrap_or(path)
}
