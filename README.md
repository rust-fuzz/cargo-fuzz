# Cargo-fuzz


[![MIT license](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)
[![Apache License 2.0](https://img.shields.io/badge/license-ALv2-blue.svg)](./LICENSE-APACHE)

[![crates.io, latest release](https://img.shields.io/crates/v/cargo-fuzz.svg)](https://crates.io/crates/cargo-fuzz)
[![crates.io, downloads](https://img.shields.io/crates/d/cargo-fuzz.svg)](https://crates.io/crates/cargo-fuzz)

[![API docs, latest release](https://docs.rs/cargo-fuzz/badge.svg)](http://docs.rs/cargo-fuzz)
[![Join the chat at https://gitter.im/djc/cargo-fuzz](https://badges.gitter.im/Join%20Chat.svg)](https://gitter.im/djc/cargo-fuzz?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

Command-line wrapper for using `libFuzzer`. Easy to use, no need to recompile LLVM!

Note: `libFuzzer` needs LLVM sanitizer support, so this is only works on x86-64 Linux and x86-64 macOS for now. This also needs a nightly since it uses some unstable command-line flags. You'll also need a C++ compiler with C++11 support.

This crate is currently under some churn -- in case stuff isn't working, please reinstall it (`cargo install cargo-fuzz -f`). Rerunning `cargo fuzz init` after moving your `fuzz` folder and updating this crate may get you a better generated `fuzz/Cargo.toml`. Expect this to settle down soon.

## Installation

```sh
$ cargo install cargo-fuzz
```

## Documentation

Documentation can be found in the [Rust Fuzz Book](https://rust-fuzz.github.io/book/cargo-fuzz.html).

## Trophy case

The trophy case has moved to a separate dedicated repository:

https://github.com/rust-fuzz/trophy-case

## License

cargo-fuzz is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](./LICENSE-APACHE) and [LICENSE-MIT](./LICENSE-MIT) for details.
