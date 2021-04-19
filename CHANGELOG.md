## Unreleased

Released YYYY-MM-DD.

### Added

* TODO (or remove section if none)

### Changed

* TODO (or remove section if none)

### Deprecated

* TODO (or remove section if none)

### Removed

* TODO (or remove section if none)

### Fixed

* TODO (or remove section if none)

### Security

* TODO (or remove section if none)

--------------------------------------------------------------------------------

## 0.10.1

Released 2020-04-19.

### Added

* Added the `--strip-dead-code` to allow stripping dead code in the linker.

  By default, dead code is linked because LLVM's code coverage instrumentation
  assumes it is present in the coverage maps for some targets. Some code bases,
  however, require stripping dead code to avoid "undefined symbol" linker
  errors. This flag allows controlling whether dead code is stipped or not in
  your build. [#260](https://github.com/rust-fuzz/cargo-fuzz/pull/260)

### Fixed

* The `cargo fuzz coverage` subcommand now passes the raw coverage files to the
  `llvm-profdata` command as a whole directory, rather than as individual files,
  which avoids an issue where too many command-line arguments were provided in
  some scenarios. [#258](https://github.com/rust-fuzz/cargo-fuzz/pull/258)

--------------------------------------------------------------------------------

## 0.10.0

Released 2021-03-10.

### Added

* Added the `cargo fuzz coverage` subcommand to generate coverage data for a
  fuzz target. Learn more in [the Coverage chapter of the Rust Fuzzing
  Book!](https://rust-fuzz.github.io/book/cargo-fuzz/coverage.html)

--------------------------------------------------------------------------------

## 0.9.2

--------------------------------------------------------------------------------

## 0.9.1

--------------------------------------------------------------------------------

## 0.9.0

--------------------------------------------------------------------------------

## 0.8.0

Released 2020-06-25.

### Changed

* `cargo fuzz build` and `cargo fuzz run` default to building with optimizations
  *and* debug assertions by default now. This is the most common configuration
  for running fuzzers, so we've made it the default. To build without
  optimizations, use the `--dev` flag, which enables Cargo's development
  profile. To build without debug assertions, use the `--release` flag, which
  enables Cargo's release profile.

### Fixed

* Building with [memory
  sanitizer](https://clang.llvm.org/docs/MemorySanitizer.html) via the
  `--sanitizer=memory` flag works correctly now! Previously, we did not rebuild
  `std` with memory sanitizer enabled, and so programs compiled with memory
  sanitizer would immediately segfault in practice.

--------------------------------------------------------------------------------

## 0.7.6

Released 2020-06-09.

### Changed

* Updated locked dependencies away from yanked versions.

--------------------------------------------------------------------------------

## 0.7.5

Released 2020-06-09.

### Added

* Added a `-v`/`--verbose` flag for enabling verbose cargo builds. This was
  always implicitly enabled before, but now is optional.
* New fuzz targets are now configured not to be tested or documented when you
  run `cargo test --all` and `cargo doc --all` and the fuzz crate is a part of a
  workspace. Previously, this caused `cargo` to accidentally start running the
  fuzzers.

### Changed

* The `-sanitizer-coverage-trace-geps` and `-sanitizer-coverage-prune-blocks=0`
  flags are not passed to LLVM anymore, as they created a lot of overhead for
  fuzz targets, without actually guiding fuzzing much.

--------------------------------------------------------------------------------

## 0.7.4

Released 2020-03-31.

### Added

* Added the `cargo fuzz fmt <target> <input>` subcommand. This prints the
  `std::fmt::Debug` output of the input. This is especially useful when the fuzz
  target takes an `Arbitrary` input type.

--------------------------------------------------------------------------------

## 0.7.3

Released 2020-02-01.

### Changed

* [Force 1 CGU when release mode is enabled](https://github.com/rust-fuzz/cargo-fuzz/pull/215)

--------------------------------------------------------------------------------

## 0.7.2

Released 2020-01-22.

### Changed

* New projects will be initialized with `libfuzzer-sys` version 0.3.0.

--------------------------------------------------------------------------------

## 0.7.1

Released 2020-01-15.

### Changed

* Updated `Cargo.lock` file's self version for `cargo-fuzz`, so that building
  doesn't change the lock file.

--------------------------------------------------------------------------------

## 0.7.0

Released 2020-01-15.

### Added

* `cargo fuzz` will show you the `Debug` output of failing inputs. This is
  particularly useful when you're using `Arbitrary` to create structured fuzz
  inputs. This requires that your fuzz target is using `libfuzzer-sys >= 0.2.0`
  from crates.io.
* `cargo fuzz` will now suggest common next tasks after finding a failing
  input. It gives you instructions on how to reproduce the failure, and how to
  run test case minimization.

### Changed

* New fuzz projects will use [`libfuzzer-sys` version
  `0.2.0`](https://github.com/rust-fuzz/libfuzzer/blob/master/CHANGELOG.md#020)
  from crates.io now, instead of a git dependency. This also pulls in
  [`arbitrary` version
  `0.3.0`](https://github.com/rust-fuzz/arbitrary/blob/master/CHANGELOG.md#030)
  and all the new goodies it contains.

--------------------------------------------------------------------------------

## 0.6.0

--------------------------------------------------------------------------------

## 0.5.0

--------------------------------------------------------------------------------

## 0.4.0

--------------------------------------------------------------------------------

## 0.3.0

--------------------------------------------------------------------------------

## 0.2.0

--------------------------------------------------------------------------------

## 0.1.0
