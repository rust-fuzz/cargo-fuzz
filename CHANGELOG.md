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
