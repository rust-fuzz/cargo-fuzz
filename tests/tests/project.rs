use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

pub fn target_tests() -> PathBuf {
    let mut path = env::current_exe().unwrap();
    path.pop(); // chop off exe name
    path.pop(); // chop off 'deps'
    path.pop(); // chop off 'debug'
    path.push("tests");
    fs::create_dir_all(&path).unwrap();
    path
}

pub fn next_root() -> PathBuf {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
    std::thread_local! {
        static TEST_ID: usize = NEXT_ID.fetch_add(1, SeqCst);
    }
    let id = TEST_ID.with(|n| *n);
    target_tests().join(format!("t{}", id))
}

pub fn project(name: &str) -> ProjectBuilder {
    ProjectBuilder::new(name, next_root(), None)
}

pub fn project_with_params(name: &str, root: PathBuf, fuzz_dir: PathBuf) -> ProjectBuilder {
    ProjectBuilder::new(name, root, Some(fuzz_dir))
}

pub struct Project {
    name: String,
    root: PathBuf,
    fuzz_dir: PathBuf,
}

pub struct ProjectBuilder {
    project: Project,
    saw_manifest: bool,
    saw_main_or_lib: bool,
}

impl ProjectBuilder {
    pub fn new(name: &str, root: PathBuf, fuzz_dir_opt: Option<PathBuf>) -> ProjectBuilder {
        println!(" ============ {} =============== ", root.display());
        drop(fs::remove_dir_all(&root));
        fs::create_dir_all(&root).unwrap();
        let fuzz_dir = fuzz_dir_opt.unwrap_or_else(|| root.join("fuzz"));
        ProjectBuilder {
            project: Project {
                name: name.to_string(),
                root,
                fuzz_dir,
            },
            saw_manifest: false,
            saw_main_or_lib: false,
        }
    }

    pub fn root(&self) -> PathBuf {
        self.project.root()
    }

    pub fn with_fuzz(&mut self) -> &mut Self {
        self.file(
            self.project.fuzz_dir.join("Cargo.toml"),
            &format!(
                r#"
                    [package]
                    name = "{name}-fuzz"
                    version = "0.0.0"
                    publish = false
                    edition = "2021"

                    [package.metadata]
                    cargo-fuzz = true

                    [workspace]
                    members = ["."]

                    [dependencies]
                    libfuzzer-sys = "0.4"

                    [dependencies.{name}]
                    path = ".."
                "#,
                name = self.project.name,
            ),
        )
    }

    pub fn fuzz_target(&mut self, name: &str, body: &str) -> &mut Self {
        let path = self.project.fuzz_target_path(name);

        let mut fuzz_cargo_toml = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(self.project.fuzz_dir.join("Cargo.toml"))
            .unwrap();
        write!(
            &mut fuzz_cargo_toml,
            r#"
                [[bin]]
                name = "{name}"
                path = {path}
                test = false
                doc = false
            "#,
            name = name,
            path = toml::to_string(&path).unwrap(),
        )
        .unwrap();

        self.file(path, body)
    }

    pub fn set_workspace_members(&mut self, members: &[&str]) -> &mut Self {
        let cargo_toml = self.root().join("Cargo.toml");
        let manifest = fs::read_to_string(cargo_toml.clone()).unwrap();

        let with_members = manifest.replace(
            "[workspace]",
            &format!(
                "[workspace]\nmembers=[{}]",
                members
                    .iter()
                    .map(|&v| format!("\"{}\"", v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );

        fs::write(cargo_toml, with_members).unwrap();
        self
    }

    pub fn file<B: AsRef<Path>>(&mut self, path: B, body: &str) -> &mut Self {
        self._file(path.as_ref(), body);
        self
    }

    fn _file(&mut self, path: &Path, body: &str) {
        if path == Path::new("Cargo.toml") {
            self.saw_manifest = true;
        }
        if path == Path::new("src").join("lib.rs") || path == Path::new("src").join("main.rs") {
            self.saw_main_or_lib = true;
        }
        let path = self.root().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(self.root().join(path), body).unwrap();
    }

    pub fn default_cargo_toml(&mut self) -> &mut Self {
        self.file(
            "Cargo.toml",
            &format!(
                r#"
                    [workspace]
                    [package]
                    name = "{name}"
                    version = "1.0.0"
                "#,
                name = self.project.name,
            ),
        )
    }

    pub fn default_src_lib(&mut self) -> &mut Self {
        self.file(
            Path::new("src").join("lib.rs"),
            r#"
                pub fn pass_fuzzing(data: &[u8]) {
                    let _ = data;
                }

                pub fn fail_fuzzing(data: &[u8]) {
                    if data.len() == 7 {
                        panic!("I'm afraid of number 7");
                    }
                }
            "#,
        )
    }

    pub fn build(&mut self) -> Project {
        if !self.saw_manifest {
            self.default_cargo_toml();
        }
        if !self.saw_main_or_lib {
            self.default_src_lib();
        }
        Project {
            fuzz_dir: self.project.fuzz_dir.clone(),
            name: self.project.name.clone(),
            root: self.project.root.clone(),
        }
    }
}

impl Project {
    pub fn root(&self) -> PathBuf {
        self.root.clone()
    }

    /// Get the build directory for the fuzz targets.
    ///
    /// This will panic if no fuzz targets have been built yet.
    pub fn fuzz_build_dir(&self) -> PathBuf {
        // Because we pass an explicit `--target` to builds, its as if we were
        // cross-compiling even when we technically aren't, and the artifacts
        // end up in `target/<triple>/*`.
        target_tests()
            .join("target")
            .read_dir()
            .expect("should get directory entries for tests' target directory")
            .map(|e| {
                e.expect("should read an entry from the tests' target directory OK")
                    .path()
            })
            .find(|d| d.is_dir() && !d.ends_with("debug") && !d.ends_with("release"))
            .unwrap()
    }

    pub fn fuzz_dir(&self) -> &Path {
        &self.fuzz_dir
    }

    pub fn fuzz_cargo_toml(&self) -> PathBuf {
        self.fuzz_dir.join("Cargo.toml")
    }

    pub fn fuzz_targets_dir(&self) -> PathBuf {
        self.fuzz_dir.join("fuzz_targets")
    }

    pub fn fuzz_target_path(&self, target: &str) -> PathBuf {
        let mut p = self.fuzz_targets_dir().join(target);
        p.set_extension("rs");
        p
    }

    pub fn fuzz_coverage_dir(&self, target: &str) -> PathBuf {
        self.fuzz_dir.join("coverage").join(target)
    }

    pub fn cargo_fuzz(&self) -> Command {
        let mut cmd = super::cargo_fuzz();
        cmd.current_dir(&self.root)
            // Even though this disables some parallelism, we won't need to
            // download and compile libbfuzzer-sys multiple times.
            .env("CARGO_HOME", target_tests().join("cargo-home"))
            .env("CARGO_TARGET_DIR", target_tests().join("target"));
        cmd
    }
}
