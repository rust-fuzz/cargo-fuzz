pub mod project;

use self::project::*;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;

fn cargo_fuzz() -> Command {
    Command::cargo_bin("cargo-fuzz").unwrap()
}

#[test]
fn help() {
    cargo_fuzz().arg("help").assert().success();
}

#[test]
fn init_with_workspace() {
    let project = project("init").build();
    project
        .cargo_fuzz()
        .arg("init")
        .arg("--fuzzing-workspace=true")
        .assert()
        .success();
    assert!(project.fuzz_dir().is_dir());
    assert!(project.fuzz_cargo_toml().is_file());
    assert!(project.fuzz_targets_dir().is_dir());
    assert!(project.fuzz_target_path("fuzz_target_1").is_file());
    project
        .cargo_fuzz()
        .arg("run")
        .arg("fuzz_target_1")
        .arg("--")
        .arg("-runs=1")
        .assert()
        .success();
}

#[test]
fn init_no_workspace() {
    let mut project_builder = project("init_no_workspace");
    let project = project_builder.build();
    project.cargo_fuzz().arg("init").assert().success();
    project_builder.set_workspace_members(&["fuzz"]);

    assert!(project.fuzz_dir().is_dir());
    assert!(project.fuzz_cargo_toml().is_file());
    assert!(project.fuzz_targets_dir().is_dir());
    assert!(project.fuzz_target_path("fuzz_target_1").is_file());
    project
        .cargo_fuzz()
        .arg("run")
        .arg("fuzz_target_1")
        .arg("--fuzz-dir")
        .arg(project.fuzz_dir().to_str().unwrap())
        .arg("--")
        .arg("-runs=1")
        .assert()
        .success();
}

#[test]
fn init_with_target_and_workspace() {
    let project = project("init_with_target").build();
    project
        .cargo_fuzz()
        .arg("init")
        .arg("-t")
        .arg("custom_target_name")
        .arg("--fuzzing-workspace=true")
        .assert()
        .success();
    assert!(project.fuzz_dir().is_dir());
    assert!(project.fuzz_cargo_toml().is_file());
    assert!(project.fuzz_targets_dir().is_dir());
    assert!(project.fuzz_target_path("custom_target_name").is_file());
    project
        .cargo_fuzz()
        .arg("run")
        .arg("custom_target_name")
        .arg("--")
        .arg("-runs=1")
        .assert()
        .success();
}

#[test]
fn init_twice() {
    let project = project("init_twice").build();

    // First init should succeed and make all the things.
    project.cargo_fuzz().arg("init").assert().success();
    assert!(project.fuzz_dir().is_dir());
    assert!(project.fuzz_cargo_toml().is_file());
    assert!(project.fuzz_targets_dir().is_dir());
    assert!(project.fuzz_target_path("fuzz_target_1").is_file());

    // Second init should fail.
    project
        .cargo_fuzz()
        .arg("init")
        .assert()
        .stderr(predicates::str::contains("File exists (os error 17)").and(
            predicates::str::contains(format!(
                "failed to create directory {}",
                project.fuzz_dir().display()
            )),
        ))
        .failure();
}

#[test]
fn init_finds_parent_project() {
    let project = project("init_finds_parent_project").build();
    project
        .cargo_fuzz()
        .current_dir(project.root().join("src"))
        .arg("init")
        .assert()
        .success();
    assert!(project.fuzz_dir().is_dir());
    assert!(project.fuzz_cargo_toml().is_file());
    assert!(project.fuzz_targets_dir().is_dir());
    assert!(project.fuzz_target_path("fuzz_target_1").is_file());
}

#[test]
fn init_defines_correct_dependency() {
    let project_name = "project_with_some_dep";
    let project = project(project_name)
        .file(
            "Cargo.toml",
            &format!(
                r#"
                    [workspace]
                    [package]
                    name = "{name}"
                    version = "1.0.0"

                    [dependencies]
                    matches = "0.1.10"
                "#,
                name = project_name
            ),
        )
        .build();
    project
        .cargo_fuzz()
        .current_dir(project.root().join("src"))
        .arg("init")
        .assert()
        .success();
    assert!(project.fuzz_dir().is_dir());
    assert!(project.fuzz_cargo_toml().is_file());
    let cargo_toml = fs::read_to_string(project.fuzz_cargo_toml()).unwrap();
    let expected_dependency_attrs =
        &format!("[dependencies.{name}]\npath = \"..\"", name = project_name);
    assert!(cargo_toml.contains(expected_dependency_attrs));
}

#[test]
fn add() {
    let project = project("add").with_fuzz().build();
    project
        .cargo_fuzz()
        .arg("add")
        .arg("new_fuzz_target")
        .assert()
        .success();
    assert!(project.fuzz_target_path("new_fuzz_target").is_file());

    assert!(project.fuzz_cargo_toml().is_file());
    let cargo_toml = fs::read_to_string(project.fuzz_cargo_toml()).unwrap();
    let expected_bin_attrs = "test = false\ndoc = false";
    assert!(cargo_toml.contains(expected_bin_attrs));

    project
        .cargo_fuzz()
        .arg("run")
        .arg("new_fuzz_target")
        .arg("--")
        .arg("-runs=1")
        .assert()
        .success();
}

#[test]
fn add_twice() {
    let project = project("add").with_fuzz().build();
    project
        .cargo_fuzz()
        .arg("add")
        .arg("new_fuzz_target")
        .assert()
        .success();
    assert!(project.fuzz_target_path("new_fuzz_target").is_file());
    project
        .cargo_fuzz()
        .arg("add")
        .arg("new_fuzz_target")
        .assert()
        .stderr(
            predicate::str::contains("could not add target")
                .and(predicate::str::contains("File exists (os error 17)")),
        )
        .failure();
}

#[test]
fn list() {
    let project = project("add").with_fuzz().build();

    // Create some targets.
    project.cargo_fuzz().arg("add").arg("c").assert().success();
    project.cargo_fuzz().arg("add").arg("b").assert().success();
    project.cargo_fuzz().arg("add").arg("a").assert().success();

    // Make sure that we can list our targets, and that they're always sorted.
    project
        .cargo_fuzz()
        .arg("list")
        .assert()
        .stdout("a\nb\nc\n")
        .success();
}

#[test]
fn run_no_crash() {
    let project = project("run_no_crash")
        .with_fuzz()
        .fuzz_target(
            "no_crash",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    run_no_crash::pass_fuzzing(data);
                });
            "#,
        )
        .build();

    project
        .cargo_fuzz()
        .arg("run")
        .arg("no_crash")
        .arg("--")
        .arg("-runs=1000")
        .assert()
        .stderr(predicate::str::contains("Done 1000 runs"))
        .success();
}

#[test]
fn run_with_crash() {
    let project = project("run_with_crash")
        .with_fuzz()
        .fuzz_target(
            "yes_crash",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    run_with_crash::fail_fuzzing(data);
                });
            "#,
        )
        .build();

    project
        .cargo_fuzz()
        .arg("run")
        .arg("yes_crash")
        .arg("--")
        .arg("-runs=1000")
        .env("RUST_BACKTRACE", "1")
        .assert()
        .stderr(
            predicate::str::contains("thread '<unnamed>' panicked at")
                .and(predicate::str::contains("I'm afraid of number 7"))
                .and(predicate::str::contains("ERROR: libFuzzer: deadly signal"))
                .and(predicate::str::contains("run_with_crash::fail_fuzzing"))
                .and(predicate::str::contains(
                    "────────────────────────────────────────────────────────────────────────────────\n\
                     \n\
                     Failing input:\n\
                     \n\
                     \tfuzz/artifacts/yes_crash/crash-"
                ))
                .and(predicate::str::contains("Output of `std::fmt::Debug`:"))
                .and(predicate::str::contains(
                    "Reproduce with:\n\
                     \n\
                     \tcargo fuzz run yes_crash fuzz/artifacts/yes_crash/crash-"
                ))
                .and(predicate::str::contains(
                    "Minimize test case with:\n\
                     \n\
                     \tcargo fuzz tmin yes_crash fuzz/artifacts/yes_crash/crash-"
                )),
        )
        .failure();
}

#[test]
fn run_with_coverage() {
    let target = "with_coverage";

    let project = project("run_with_coverage")
        .with_fuzz()
        .fuzz_target(
            target,
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    println!("{:?}", data);
                });
            "#,
        )
        .build();

    project
        .cargo_fuzz()
        .arg("run")
        .arg(target)
        .arg("--")
        .arg("-runs=100")
        .assert()
        .stderr(predicate::str::contains("Done 100 runs"))
        .success();

    project
        .cargo_fuzz()
        .arg("coverage")
        .arg(target)
        .assert()
        .stderr(predicate::str::contains("Coverage data merged and saved"))
        .success();

    let profdata_file = project.fuzz_coverage_dir(target).join("coverage.profdata");
    assert!(profdata_file.exists(), "Coverage data file not generated");
}

#[test]
fn run_without_sanitizer_with_crash() {
    let project = project("run_without_sanitizer_with_crash")
        .with_fuzz()
        .fuzz_target(
            "yes_crash",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    run_without_sanitizer_with_crash::fail_fuzzing(data);
                });
            "#,
        )
        .build();

    project
        .cargo_fuzz()
        .arg("run")
        .arg("yes_crash")
        .arg("--")
        .arg("-runs=1000")
        .arg("-sanitizer=none")
        .env("RUST_BACKTRACE", "1")
        .assert()
        .stderr(
            predicate::str::contains("thread '<unnamed>' panicked at")
                .and(predicate::str::contains("I'm afraid of number 7"))
                .and(predicate::str::contains("ERROR: libFuzzer: deadly signal"))
                .and(predicate::str::contains("run_without_sanitizer_with_crash::fail_fuzzing"))
                .and(predicate::str::contains(
                    "────────────────────────────────────────────────────────────────────────────────\n\
                     \n\
                     Failing input:\n\
                     \n\
                     \tfuzz/artifacts/yes_crash/crash-"
                ))
                .and(predicate::str::contains("Output of `std::fmt::Debug`:"))
                .and(predicate::str::contains(
                    "Reproduce with:\n\
                     \n\
                     \tcargo fuzz run yes_crash fuzz/artifacts/yes_crash/crash-"
                ))
                .and(predicate::str::contains(
                    "Minimize test case with:\n\
                     \n\
                     \tcargo fuzz tmin yes_crash fuzz/artifacts/yes_crash/crash-"
                )),
        )
        .failure();
}

// TODO: these msan tests are crashing `rustc` in CI:
// https://github.com/rust-fuzz/cargo-fuzz/issues/323
//
// #[test]
// fn run_with_msan_no_crash() {
//     let project = project("run_with_msan_no_crash")
//         .with_fuzz()
//         .fuzz_target(
//             "msan_no_crash",
//             r#"
//                 #![no_main]
//                 use libfuzzer_sys::fuzz_target;
//
//                 fuzz_target!(|data: &[u8]| {
//                     // get data from fuzzer and print it
//                     // to force a memory access that cannot be optimized out
//                     if let Some(x) = data.get(0) {
//                         dbg!(x);
//                     }
//                 });
//             "#,
//         )
//         .build();
//
//     project
//         .cargo_fuzz()
//         .arg("run")
//         .arg("--sanitizer=memory")
//         .arg("msan_no_crash")
//         .arg("--")
//         .arg("-runs=1000")
//         .assert()
//         .stderr(predicate::str::contains("Done 1000 runs"))
//         .success();
// }
//
// #[test]
// fn run_with_msan_with_crash() {
//     let project = project("run_with_msan_with_crash")
//         .with_fuzz()
//         .fuzz_target(
//             "msan_with_crash",
//             r#"
//                 #![no_main]
//                 use libfuzzer_sys::fuzz_target;
//
//                 fuzz_target!(|data: &[u8]| {
//                     let test_data: Vec<u8> = Vec::with_capacity(4);
//                     let uninitialized_value = unsafe {test_data.get_unchecked(0)};
//                     // prevent uninit read from being optimized out
//                     println!("{}", uninitialized_value);
//                 });
//             "#,
//         )
//         .build();
//
//     project
//         .cargo_fuzz()
//         .arg("run")
//         .arg("--sanitizer=memory")
//         .arg("msan_with_crash")
//         .arg("--")
//         .arg("-runs=1000")
//         .assert()
//         .stderr(
//             predicate::str::contains("MemorySanitizer: use-of-uninitialized-value")
//                 .and(predicate::str::contains(
//                     "Reproduce with:\n\
//                 \n\
//                 \tcargo fuzz run --sanitizer=memory msan_with_crash fuzz/artifacts/msan_with_crash/crash-",
//                 ))
//                 .and(predicate::str::contains(
//                     "Minimize test case with:\n\
//                 \n\
//                 \tcargo fuzz tmin --sanitizer=memory msan_with_crash fuzz/artifacts/msan_with_crash/crash-",
//                 )),
//         )
//         .failure();
// }

#[test]
fn run_one_input() {
    let corpus = Path::new("fuzz").join("corpus").join("run_one");

    let project = project("run_one_input")
        .with_fuzz()
        .fuzz_target(
            "run_one",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    assert!(data.is_empty());
                });
            "#,
        )
        .file(corpus.join("pass"), "")
        .file(corpus.join("fail"), "not empty")
        .build();

    project
        .cargo_fuzz()
        .arg("run")
        .arg("run_one")
        .arg(corpus.join("pass"))
        .assert()
        .stderr(
            predicate::str::contains("Running 1 inputs 1 time(s) each.").and(
                predicate::str::contains("Running: fuzz/corpus/run_one/pass"),
            ),
        )
        .success();
}

#[test]
fn run_a_few_inputs() {
    let corpus = Path::new("fuzz").join("corpus").join("run_few");

    let project = project("run_a_few_inputs")
        .with_fuzz()
        .fuzz_target(
            "run_few",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    assert!(data.len() != 4);
                });
            "#,
        )
        .file(corpus.join("pass-0"), "")
        .file(corpus.join("pass-1"), "1")
        .file(corpus.join("pass-2"), "12")
        .file(corpus.join("pass-3"), "123")
        .file(corpus.join("fail"), "fail")
        .build();

    project
        .cargo_fuzz()
        .arg("run")
        .arg("run_few")
        .arg(corpus.join("pass-0"))
        .arg(corpus.join("pass-1"))
        .arg(corpus.join("pass-2"))
        .arg(corpus.join("pass-3"))
        .assert()
        .stderr(
            predicate::str::contains("Running 4 inputs 1 time(s) each.").and(
                predicate::str::contains("Running: fuzz/corpus/run_few/pass"),
            ),
        )
        .success();
}

#[test]
fn run_alt_corpus() {
    let corpus = Path::new("fuzz").join("corpus").join("run_alt");
    let alt_corpus = Path::new("fuzz").join("alt-corpus").join("run_alt");

    let project = project("run_alt_corpus")
        .with_fuzz()
        .fuzz_target(
            "run_alt",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    assert!(data.len() <= 1);
                });
            "#,
        )
        .file(corpus.join("fail"), "fail")
        .file(alt_corpus.join("pass-0"), "0")
        .file(alt_corpus.join("pass-1"), "1")
        .file(alt_corpus.join("pass-2"), "2")
        .build();

    project
        .cargo_fuzz()
        .arg("run")
        .arg("run_alt")
        .arg(&alt_corpus)
        .arg("--")
        .arg("-runs=0")
        .assert()
        .stderr(
            predicate::str::contains("3 files found in fuzz/alt-corpus/run_alt")
                .and(predicate::str::contains("fuzz/corpus/run_alt").not())
                // libFuzzer will always test the empty input, so the number of
                // runs performed is always one more than the number of files in
                // the corpus.
                .and(predicate::str::contains("Done 4 runs in")),
        )
        .success();
}

#[test]
fn debug_fmt() {
    let corpus = Path::new("fuzz").join("corpus").join("debugfmt");
    let project = project("debugfmt")
        .with_fuzz()
        .fuzz_target(
            "debugfmt",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;
                use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured, Result};

                #[derive(Debug)]
                pub struct Rgb {
                    r: u8,
                    g: u8,
                    b: u8,
                }

                impl<'a> Arbitrary<'a> for Rgb {
                    fn arbitrary(raw: &mut Unstructured<'a>) -> Result<Self> {
                        let mut buf = [0; 3];
                        raw.fill_buffer(&mut buf)?;
                        let r = buf[0];
                        let g = buf[1];
                        let b = buf[2];
                        Ok(Rgb { r, g, b })
                    }
                }

                fuzz_target!(|data: Rgb| {
                    let _ = data;
                });
            "#,
        )
        .file(corpus.join("0"), "111")
        .build();

    project
        .cargo_fuzz()
        .arg("fmt")
        .arg("debugfmt")
        .arg("fuzz/corpus/debugfmt/0")
        .assert()
        .stderr(predicates::str::contains(
            "
Rgb {
    r: 49,
    g: 49,
    b: 49,
}",
        ))
        .success();
}

#[test]
fn cmin() {
    let corpus = Path::new("fuzz").join("corpus").join("foo");
    let project = project("cmin")
        .with_fuzz()
        .fuzz_target(
            "foo",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    let _ = data;
                });
            "#,
        )
        .file(corpus.join("0"), "")
        .file(corpus.join("1"), "a")
        .file(corpus.join("2"), "ab")
        .file(corpus.join("3"), "abc")
        .file(corpus.join("4"), "abcd")
        .build();

    let corpus_count = || {
        fs::read_dir(project.root().join("fuzz").join("corpus").join("foo"))
            .unwrap()
            .count()
    };
    assert_eq!(corpus_count(), 5);

    project
        .cargo_fuzz()
        .arg("cmin")
        .arg("foo")
        .assert()
        .success();
    assert_eq!(corpus_count(), 1);
}

#[test]
fn tmin() {
    let corpus = Path::new("fuzz").join("corpus").join("i_hate_zed");
    let test_case = corpus.join("test-case");
    let project = project("tmin")
        .with_fuzz()
        .fuzz_target(
            "i_hate_zed",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    let s = String::from_utf8_lossy(data);
                    if s.contains('z') {
                        panic!("nooooooooo");
                    }
                });
            "#,
        )
        .file(&test_case, "pack my box with five dozen liquor jugs")
        .build();
    let test_case = project.root().join(test_case);
    project
        .cargo_fuzz()
        .arg("tmin")
        .arg("i_hate_zed")
        .arg("--sanitizer=none")
        .arg(&test_case)
        .assert()
        .stderr(
            predicates::str::contains("CRASH_MIN: minimizing crash input: ")
                .and(predicate::str::contains("(1 bytes) caused a crash"))
                .and(predicate::str::contains(
                    "────────────────────────────────────────────────────────────────────────────────\n\
                     \n\
                     Minimized artifact:\n\
                     \n\
                     \tfuzz/artifacts/i_hate_zed/minimized-from-"))
                .and(predicate::str::contains(
                    "Reproduce with:\n\
                     \n\
                     \tcargo fuzz run --sanitizer=none i_hate_zed fuzz/artifacts/i_hate_zed/minimized-from-"
                )),
        )
        .success();
}

#[test]
fn build_all() {
    let project = project("build_all").with_fuzz().build();

    // Create some targets.
    project
        .cargo_fuzz()
        .arg("add")
        .arg("build_all_a")
        .assert()
        .success();
    project
        .cargo_fuzz()
        .arg("add")
        .arg("build_all_b")
        .assert()
        .success();

    // Build to ensure that the build directory is created and
    // `fuzz_build_dir()` won't panic.
    project.cargo_fuzz().arg("build").assert().success();

    let build_dir = project.fuzz_build_dir().join("release");

    let a_bin = build_dir.join("build_all_a");
    let b_bin = build_dir.join("build_all_b");

    // Remove the files we just built.
    fs::remove_file(&a_bin).unwrap();
    fs::remove_file(&b_bin).unwrap();

    assert!(!a_bin.is_file());
    assert!(!b_bin.is_file());

    // Test that building all fuzz targets does in fact recreate the files.
    project.cargo_fuzz().arg("build").assert().success();

    assert!(a_bin.is_file());
    assert!(b_bin.is_file());
}

#[test]
fn build_one() {
    let project = project("build_one").with_fuzz().build();

    // Create some targets.
    project
        .cargo_fuzz()
        .arg("add")
        .arg("build_one_a")
        .assert()
        .success();
    project
        .cargo_fuzz()
        .arg("add")
        .arg("build_one_b")
        .assert()
        .success();

    // Build to ensure that the build directory is created and
    // `fuzz_build_dir()` won't panic.
    project.cargo_fuzz().arg("build").assert().success();

    let build_dir = project.fuzz_build_dir().join("release");
    let a_bin = build_dir.join("build_one_a");
    let b_bin = build_dir.join("build_one_b");

    // Remove the files we just built.
    fs::remove_file(&a_bin).unwrap();
    fs::remove_file(&b_bin).unwrap();

    assert!(!a_bin.is_file());
    assert!(!b_bin.is_file());

    // Test that we can build one and not the other.
    project
        .cargo_fuzz()
        .arg("build")
        .arg("build_one_a")
        .assert()
        .success();

    assert!(a_bin.is_file());
    assert!(!b_bin.is_file());
}

#[test]
fn build_dev() {
    let project = project("build_dev").with_fuzz().build();

    // Create some targets.
    project
        .cargo_fuzz()
        .arg("add")
        .arg("build_dev_a")
        .assert()
        .success();
    project
        .cargo_fuzz()
        .arg("add")
        .arg("build_dev_b")
        .assert()
        .success();

    // Build to ensure that the build directory is created and
    // `fuzz_build_dir()` won't panic.
    project
        .cargo_fuzz()
        .arg("build")
        .arg("--dev")
        .assert()
        .success();

    let build_dir = project.fuzz_build_dir().join("debug");

    let a_bin = build_dir.join("build_dev_a");
    let b_bin = build_dir.join("build_dev_b");

    // Remove the files we just built.
    fs::remove_file(&a_bin).unwrap();
    fs::remove_file(&b_bin).unwrap();

    assert!(!a_bin.is_file());
    assert!(!b_bin.is_file());

    // Test that building all fuzz targets does in fact recreate the files.
    project
        .cargo_fuzz()
        .arg("build")
        .arg("--dev")
        .assert()
        .success();

    assert!(a_bin.is_file());
    assert!(b_bin.is_file());
}

#[test]
fn build_stripping_dead_code() {
    let project = project("build_strip").with_fuzz().build();

    // Create some targets.
    project
        .cargo_fuzz()
        .arg("add")
        .arg("build_strip_a")
        .assert()
        .success();

    project
        .cargo_fuzz()
        .arg("build")
        .arg("--strip-dead-code")
        .arg("--dev")
        .assert()
        .success();

    let build_dir = project.fuzz_build_dir().join("debug");

    let a_bin = build_dir.join("build_strip_a");
    assert!(a_bin.is_file(), "Not a file: {}", a_bin.display());
}

#[test]
fn run_with_different_fuzz_dir() {
    let (fuzz_dir, mut project_builder) = project_with_fuzz_dir(
        "project_likes_to_move_it",
        Some("dir_likes_to_move_it_move_it"),
    );
    let project = project_builder
        .with_fuzz()
        .fuzz_target(
            "you_like_to_move_it",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|_data: &[u8]| {
                });
            "#,
        )
        .build();

    project
        .cargo_fuzz()
        .arg("run")
        .arg("--fuzz-dir")
        .arg(fuzz_dir)
        .arg("you_like_to_move_it")
        .arg("--")
        .arg("-runs=1")
        .assert()
        .stderr(predicate::str::contains("Done 2 runs"))
        .success();
}

#[test]
fn run_diagnostic_contains_fuzz_dir() {
    let (fuzz_dir, mut project_builder) = project_with_fuzz_dir("run_with_crash", None);
    let project = project_builder
        .with_fuzz()
        .fuzz_target(
            "yes_crash",
            r#"
                #![no_main]
                use libfuzzer_sys::fuzz_target;

                fuzz_target!(|data: &[u8]| {
                    run_with_crash::fail_fuzzing(data);
                });
            "#,
        )
        .build();

    let run = format!(
        "cargo fuzz run --fuzz-dir {} yes_crash custom_dir/artifacts/yes_crash",
        &fuzz_dir
    );

    let tmin = format!(
        "cargo fuzz tmin --fuzz-dir {} yes_crash custom_dir/artifacts/yes_crash",
        &fuzz_dir
    );

    project
        .cargo_fuzz()
        .arg("run")
        .arg("--fuzz-dir")
        .arg(fuzz_dir)
        .arg("yes_crash")
        .arg("--")
        .arg("-runs=1000")
        .assert()
        .stderr(predicates::str::contains(run).and(predicate::str::contains(tmin)))
        .failure();
}

fn project_with_fuzz_dir(
    project_name: &str,
    fuzz_dir_opt: Option<&str>,
) -> (String, ProjectBuilder) {
    let fuzz_dir = fuzz_dir_opt.unwrap_or("custom_dir");
    let next_root = next_root();
    let fuzz_dir_pb = next_root.join(fuzz_dir);
    let fuzz_dir_sting = fuzz_dir_pb.display().to_string();
    let pb = project_with_params(project_name, next_root, fuzz_dir_pb);
    (fuzz_dir_sting, pb)
}
