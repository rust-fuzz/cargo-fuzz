use crate::options::{self, BuildOptions, Sanitizer};
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

pub struct FuzzProject {
    /// Path to the root cargo project
    ///
    /// Not the project with fuzz targets, but the project being fuzzed
    root_project: PathBuf,
    targets: Vec<String>,
}

impl FuzzProject {
    /// Find an existing `cargo fuzz` project by starting at the current
    /// directory and walking up the filesystem.
    pub fn find_existing() -> Result<Self> {
        let mut project = FuzzProject {
            root_project: find_package()?,
            targets: Vec::new(),
        };
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

    /// Create the fuzz project structure
    ///
    /// This will not clone libfuzzer-sys
    pub fn init(init: &options::Init) -> Result<Self> {
        let project = FuzzProject {
            root_project: find_package()?,
            targets: Vec::new(),
        };
        let fuzz_project = project.path();
        let root_project_name = project.root_project_name()?;

        // TODO: check if the project is already initialized
        fs::create_dir(&fuzz_project)
            .with_context(|| format!("failed to create directory {}", fuzz_project.display()))?;

        let fuzz_targets_dir = fuzz_project.join(crate::FUZZ_TARGETS_DIR);
        fs::create_dir(&fuzz_targets_dir).with_context(|| {
            format!("failed to create directory {}", fuzz_targets_dir.display())
        })?;

        let cargo_toml = fuzz_project.join("Cargo.toml");
        let mut cargo = fs::File::create(&cargo_toml)
            .with_context(|| format!("failed to create {}", cargo_toml.display()))?;
        cargo
            .write_fmt(toml_template!(root_project_name))
            .with_context(|| format!("failed to write to {}", cargo_toml.display()))?;

        let gitignore = fuzz_project.join(".gitignore");
        let mut ignore = fs::File::create(&gitignore)
            .with_context(|| format!("failed to create {}", gitignore.display()))?;
        ignore
            .write_fmt(gitignore_template!())
            .with_context(|| format!("failed to write to {}", gitignore.display()))?;

        project
            .create_target_template(&init.target)
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
    pub fn add_target(&self, add: &options::Add) -> Result<()> {
        // Create corpus and artifact directories for the newly added target
        self.corpus_for(&add.target)?;
        self.artifacts_for(&add.target)?;
        self.create_target_template(&add.target)
            .with_context(|| format!("could not add target {:?}", add.target))
    }

    /// Add a new fuzz target script with a given name
    fn create_target_template(&self, target: &str) -> Result<()> {
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
        script.write_fmt(target_template!())?;

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
            .arg("--verbose")
            // --target=<TARGET> won't pass rustflags to build scripts
            .arg("--target")
            .arg(&build.triple);
        if build.release {
            cmd.arg("--release");
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

        let mut rustflags: String = "--cfg fuzzing \
                                     -Cpasses=sancov \
                                     -Cllvm-args=-sanitizer-coverage-level=4 \
                                     -Cllvm-args=-sanitizer-coverage-trace-compares \
                                     -Cllvm-args=-sanitizer-coverage-inline-8bit-counters \
                                     -Cllvm-args=-sanitizer-coverage-trace-geps \
                                     -Cllvm-args=-sanitizer-coverage-prune-blocks=0 \
                                     -Cllvm-args=-sanitizer-coverage-pc-table \
                                     -Clink-dead-code"
            .to_owned();
        match build.sanitizer {
            Sanitizer::None => {}
            _ => rustflags.push_str(&format!(
                " -Zsanitizer={sanitizer}",
                sanitizer = build.sanitizer
            )),
        }
        if build.triple.contains("-linux-") {
            rustflags.push_str(" -Cllvm-args=-sanitizer-coverage-stack-depth");
        }
        if build.debug_assertions {
            rustflags.push_str(" -Cdebug-assertions");
        }

        // If release mode is enabled then we force 1 CGU to be used in rustc.
        // This will result in slower compilations but it looks like the sancov
        // passes otherwise add `notEligibleToImport` annotations to functions
        // in LLVM IR, meaning that *nothing* can get imported with ThinLTO.
        // This means that in release mode, where ThinLTO is critical for
        // performance, we're taking a huge hit relative to actual release mode.
        // Local tests have once showed this to be a ~3x faster runtime where
        // otherwise functions like `Vec::as_ptr` aren't inlined.
        if build.release {
            rustflags.push_str(" -C codegen-units=1");
        }

        if let Ok(other_flags) = env::var("RUSTFLAGS") {
            rustflags.push_str(" ");
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

        let mut artifact_arg = ffi::OsString::from("-artifact_prefix=");
        artifact_arg.push(self.artifacts_for(&fuzz_target)?);
        cmd.arg("--").arg(artifact_arg);

        Ok(cmd)
    }

    pub fn exec_build(
        &self,
        build: &options::BuildOptions,
        fuzz_target: Option<&str>,
    ) -> Result<()> {
        let mut cmd = self.cargo("build", build)?;

        if let Some(fuzz_target) = fuzz_target {
            cmd.arg("--bin").arg(fuzz_target);
        } else {
            cmd.arg("--bins");
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

        let mut cmd = self.cargo_run(&build, &target)?;
        cmd.stdin(Stdio::null());
        cmd.env("RUST_LIBFUZZER_DEBUG_PATH", &debug_output.path());
        cmd.arg(&artifact);

        let output = cmd
            .output()
            .with_context(|| format!("failed to run command: {:?}", cmd))?;

        if !output.status.success() {
            bail!(
                "Fuzz target '{target}' exited with failure when attemping to \
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

    /// Fuzz a given fuzz target
    pub fn exec_fuzz(&self, run: &options::Run) -> Result<()> {
        self.exec_build(&run.build, Some(&run.target))?;
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

            eprintln!(
                "Reproduce with:\n\n\tcargo fuzz run {target} {artifact}\n",
                target = &run.target,
                artifact = artifact.display()
            );
            eprintln!(
                "Minimize test case with:\n\n\tcargo fuzz tmin {target} {artifact}\n",
                target = &run.target,
                artifact = artifact.display()
            );
        }

        eprintln!("{:─<80}\n", "");
        bail!("Fuzz target exited with {}", status)
    }

    pub fn exec_tmin(&self, tmin: &options::Tmin) -> Result<()> {
        self.exec_build(&tmin.build, Some(&tmin.target))?;
        let mut cmd = self.cargo_run(&tmin.build, &tmin.target)?;
        cmd.arg("-minimize_crash=1")
            .arg(format!("-runs={}", tmin.runs))
            .arg(&tmin.test_case);

        let before_tmin = time::SystemTime::now();

        let mut child = cmd
            .spawn()
            .with_context(|| format!("failed to spawn command: {:?}", cmd))?;
        let status = child
            .wait()
            .with_context(|| format!("failed to wait on child process for command: {:?}", cmd))?;
        if !status.success() {
            eprintln!("\n{:─<80}\n", "");
            return Err(anyhow!("Command `{:?}` exited with {}", cmd, status))
                .with_context(|| {
                    "Test case minimization failed.\n\
                     \n\
                     Usually this isn't a hard error, and just means that libfuzzer\n\
                     doesn't know how to minimize the test case any further while\n\
                     still reproducing the original crash.\n\
                     \n\
                     See the logs above for details."
                })
                .into();
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

            eprintln!(
                "Reproduce with:\n\n\tcargo fuzz run {target} {artifact}\n",
                target = &tmin.target,
                artifact = artifact.display()
            );
        }

        Ok(())
    }

    pub fn exec_cmin(&self, cmin: &options::Cmin) -> Result<()> {
        self.exec_build(&cmin.build, Some(&cmin.target))?;
        let mut cmd = self.cargo_run(&cmin.build, &cmin.target)?;

        let corpus = if let Some(corpus) = cmin.corpus.clone() {
            corpus
        } else {
            self.corpus_for(&cmin.target)?
        };
        let corpus = corpus
            .to_str()
            .ok_or_else(|| anyhow!("corpus must be valid unicode"))?
            .to_owned();

        let tmp = tempfile::TempDir::new_in(self.path())?;
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

    fn path(&self) -> PathBuf {
        self.root_project.join("fuzz")
    }

    fn manifest_path(&self) -> PathBuf {
        self.path().join("Cargo.toml")
    }

    fn corpus_for(&self, target: &str) -> Result<PathBuf> {
        let mut p = self.path();
        p.push("corpus");
        p.push(target);
        fs::create_dir_all(&p)
            .with_context(|| format!("could not make a corpus directory at {:?}", p))?;
        Ok(p)
    }

    fn artifacts_for(&self, target: &str) -> Result<PathBuf> {
        let mut p = self.path();
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
        let mut root = self.path();
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

    fn root_project_name(&self) -> Result<String> {
        let filename = self.root_project.join("Cargo.toml");
        let mut file = fs::File::open(&filename)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let value: toml::Value = toml::from_slice(&data)?;
        let name = value
            .as_table()
            .and_then(|v| v.get("package"))
            .and_then(toml::Value::as_table)
            .and_then(|v| v.get("name"))
            .and_then(toml::Value::as_str);
        if let Some(name) = name {
            Ok(String::from(name))
        } else {
            bail!("{} (package.name) is malformed", filename.display());
        }
    }
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
        .unwrap_or(&path)
}
